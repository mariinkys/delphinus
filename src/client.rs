//! Browser-only helpers. They compile on the server too but are only ever
//! called from event handlers and effects, which never run there

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

fn window() -> Option<web_sys::Window> {
    web_sys::window()
}

// Local storage

pub fn storage_get(key: &str) -> Option<String> {
    window()?.local_storage().ok()??.get_item(key).ok()?
}

pub fn storage_set(key: &str, value: &str) {
    if let Some(Ok(Some(s))) = window().map(|w| w.local_storage()) {
        let _ = s.set_item(key, value);
    }
}

// Theme

pub fn is_dark() -> bool {
    let Some(w) = window() else { return false };
    let explicit = w
        .document()
        .and_then(|d| d.document_element())
        .and_then(|e| e.get_attribute("data-theme"));
    match explicit.as_deref() {
        Some("dark") => true,
        Some("light") => false,
        _ => w
            .match_media("(prefers-color-scheme: dark)")
            .ok()
            .flatten()
            .is_some_and(|m| m.matches()),
    }
}

pub fn set_theme(dark: bool) {
    let theme = if dark { "dark" } else { "light" };
    if let Some(e) = window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = e.set_attribute("data-theme", theme);
    }
    storage_set("delphinus.theme", theme);
}

// Clipboard and downloads

pub async fn copy_text(text: &str) -> bool {
    let Some(w) = window() else { return false };
    let promise = w.navigator().clipboard().write_text(text);
    JsFuture::from(promise).await.is_ok()
}

pub fn download(file_name: &str, mime: &str, text: &str) -> bool {
    let run = || -> Result<(), JsValue> {
        let document = window().and_then(|w| w.document()).ok_or(JsValue::NULL)?;
        let parts = js_sys::Array::of1(&JsValue::from_str(text));
        let opts = web_sys::BlobPropertyBag::new();
        opts.set_type(mime);
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)?;
        let a: web_sys::HtmlAnchorElement = document.create_element("a")?.dyn_into()?;
        a.set_href(&url);
        a.set_download(file_name);
        a.click();
        web_sys::Url::revoke_object_url(&url)?;
        Ok(())
    };
    run().is_ok()
}

// Image scanning

/// Longest side sent to the server. Phone photos are scaled down in the
/// browser first, which keeps the upload small and the server's peak memory low
const UPLOAD_MAX_SIDE: f64 = 1800.0;

pub fn is_supported_image(file: &web_sys::File) -> bool {
    matches!(
        file.type_().as_str(),
        "image/png" | "image/jpeg" | "image/jpg" | "image/webp"
    )
}

/// Scales the image down (if needed), uploads it and returns the CJK lines found
pub async fn scan_image(file: web_sys::File) -> Result<Vec<String>, String> {
    let body: web_sys::Blob = match shrink(&file).await {
        Ok(Some(blob)) => blob,
        // Small enough already, or the browser cannot re-encode: send as is
        _ => file.into(),
    };

    let w = window().ok_or("no window")?;
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_body(&body);
    let resp: web_sys::Response = JsFuture::from(w.fetch_with_str_and_init("/api/ocr", &init))
        .await
        .map_err(|_| "Couldn't reach the server. Check your connection and try again.".to_string())?
        .dyn_into()
        .map_err(|_| "Unexpected response from the server.".to_string())?;
    let text = JsFuture::from(resp.text().map_err(|_| "Unreadable response.")?)
        .await
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();

    #[derive(serde::Deserialize)]
    struct Reply {
        #[serde(default)]
        lines: Vec<String>,
        #[serde(default)]
        error: Option<String>,
    }
    let reply: Reply = serde_json::from_str(&text).map_err(|_| {
        if resp.status() == 413 {
            "That image is too large. Crop it to the words you need.".to_string()
        } else {
            format!("Scanning failed (error {}).", resp.status())
        }
    })?;
    match reply.error {
        Some(e) => Err(e),
        None => Ok(reply.lines),
    }
}

async fn shrink(file: &web_sys::File) -> Result<Option<web_sys::Blob>, JsValue> {
    let w = window().ok_or(JsValue::NULL)?;
    let bitmap: web_sys::ImageBitmap = JsFuture::from(w.create_image_bitmap_with_blob(file)?)
        .await?
        .dyn_into()?;
    let (bw, bh) = (bitmap.width() as f64, bitmap.height() as f64);
    let scale = UPLOAD_MAX_SIDE / bw.max(bh);
    if scale >= 1.0 && file.size() < 1_500_000.0 {
        bitmap.close();
        return Ok(None);
    }
    let scale = scale.min(1.0);
    let (tw, th) = ((bw * scale).round().max(1.0), (bh * scale).round().max(1.0));
    let canvas = web_sys::OffscreenCanvas::new(tw as u32, th as u32)?;
    let ctx: web_sys::OffscreenCanvasRenderingContext2d =
        canvas.get_context("2d")?.ok_or(JsValue::NULL)?.dyn_into()?;
    ctx.draw_image_with_image_bitmap_and_dw_and_dh(&bitmap, 0.0, 0.0, tw, th)?;
    bitmap.close();
    let opts = web_sys::ImageEncodeOptions::new();
    opts.set_type("image/jpeg");
    opts.set_quality(0.9);
    let blob: web_sys::Blob = JsFuture::from(canvas.convert_to_blob_with_options(&opts)?)
        .await?
        .dyn_into()?;
    Ok(Some(blob))
}

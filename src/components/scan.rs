use super::layout::Notice;
use leptos::{html, prelude::*, task::spawn_local};

/// Sends an image to the server's OCR and hands back the lines it found
pub fn scan_file(
    file: web_sys::File,
    busy: RwSignal<bool>,
    on_lines: Callback<Vec<String>>,
    notice: Notice,
) {
    if !crate::client::is_supported_image(&file) {
        notice.error("Use a PNG, JPEG or WebP image.");
        return;
    }
    if busy.get_untracked() {
        return;
    }
    busy.set(true);
    spawn_local(async move {
        match crate::client::scan_image(file).await {
            Ok(lines) if lines.is_empty() => {
                notice.error("No Chinese or Japanese text found in that image.")
            }
            Ok(lines) => {
                let n = lines.len();
                on_lines.run(lines);
                notice.info(format!(
                    "Added {n} {} from the image.",
                    if n == 1 { "line" } else { "lines" }
                ));
            }
            Err(e) => notice.error(e),
        }
        busy.set(false);
    });
}

#[component]
pub fn ScanButton(busy: RwSignal<bool>, on_lines: Callback<Vec<String>>) -> impl IntoView {
    let notice = Notice::use_();
    let input = NodeRef::<html::Input>::new();
    view! {
        <input
            node_ref=input
            class="visually-hidden"
            type="file"
            accept="image/png,image/jpeg,image/webp"
            tabindex="-1"
            aria-hidden="true"
            on:change=move |_| {
                if let Some(el) = input.get() {
                    if let Some(file) = el.files().and_then(|f| f.get(0)) {
                        scan_file(file, busy, on_lines, notice);
                    }
                    el.set_value("");
                }
            }
        />
        <button
            type="button"
            class="button"
            disabled=busy
            on:click=move |_| {
                if let Some(el) = input.get() {
                    el.click();
                }
            }
        >
            {move || if busy.get() { "Reading image…" } else { "Scan an image" }}
        </button>
    }
}

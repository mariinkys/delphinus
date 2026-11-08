//! Image-to-text for pasted screenshots and photos.
//!
//! The OCR models need far more memory (~150 MB) than the rest of the
//! server combined, so they never live in the server process:
//! * a worker process starts on the first request,
//! * only one image is processed at a time (bounded peak memory),
//! * after `DELPHINUS_OCR_IDLE_SECS` (default 300) without use the worker
//!   exits, returning all of its memory to the system.
//!
//! Built without the `ocr` feature, the endpoint answers 503.

use axum::{Json, body::Bytes, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct OcrReply {
    lines: Vec<String>,
}

#[derive(Serialize)]
struct OcrError {
    error: String,
}

fn fail(code: StatusCode, msg: impl Into<String>) -> axum::response::Response {
    (code, Json(OcrError { error: msg.into() })).into_response()
}

/// `POST /api/ocr` with the raw image as the body.
pub async fn handler(body: Bytes) -> axum::response::Response {
    if body.is_empty() {
        return fail(StatusCode::BAD_REQUEST, "The request had no image.");
    }
    #[cfg(feature = "ocr")]
    {
        match engine::recognize(body).await {
            Ok(lines) => Json(OcrReply { lines }).into_response(),
            Err(e) => fail(e.0, e.1),
        }
    }
    #[cfg(not(feature = "ocr"))]
    {
        let _ = OcrReply { lines: Vec::new() };
        fail(
            StatusCode::SERVICE_UNAVAILABLE,
            "Image scanning is not available on this server.",
        )
    }
}

pub fn enabled() -> bool {
    #[cfg(feature = "ocr")]
    {
        engine::enabled()
    }
    #[cfg(not(feature = "ocr"))]
    {
        false
    }
}

#[cfg(feature = "ocr")]
pub use engine::init;

#[cfg(feature = "ocr")]
pub use engine::worker_main;

#[cfg(feature = "ocr")]
mod engine {
    //! The models run in a child process (`delphinus ocr-worker`), started on
    //! the first scan and stopped after being idle. Ending the process is the
    //! only way to give *all* of ONNX Runtime's memory back to the system:
    //! unloading inside the server still left ~40 MB behind.
    //!
    //! Protocol over the child's stdin/stdout: the server writes a `u32`
    //! little-endian length followed by the image bytes; the worker answers
    //! with one JSON line, `{"lines":[…]}` or `{"error":"…","status":422}`.

    use axum::{body::Bytes, http::StatusCode};
    use std::{
        io::{BufRead, BufReader, Read, Write},
        path::{Path, PathBuf},
        process::{Child, ChildStdin, ChildStdout, Command, Stdio},
        sync::{Mutex, OnceLock},
        time::{Duration, Instant},
    };
    use tokio::sync::Semaphore;

    /// Longest side fed to the models; larger images are scaled down first.
    const MAX_SIDE: u32 = 2048;
    /// Refuse to decode anything bigger than this (pixels), before allocating.
    const MAX_PIXELS: u64 = 40_000_000;
    const FILES: [&str; 3] = [
        "pp-ocrv5_mobile_det.onnx",
        "pp-ocrv5_mobile_rec.onnx",
        "ppocrv5_dict.txt",
    ];

    struct Worker {
        child: Child,
        stdin: ChildStdin,
        stdout: BufReader<ChildStdout>,
    }

    impl Drop for Worker {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    struct Service {
        dir: PathBuf,
        idle: Duration,
        worker: Mutex<Option<Worker>>,
        last_used: Mutex<Instant>,
        permit: Semaphore,
    }

    static SERVICE: OnceLock<Service> = OnceLock::new();

    pub fn enabled() -> bool {
        SERVICE.get().is_some()
    }

    /// Registers the service if the model files exist. Nothing is started yet.
    pub fn init(dir: PathBuf, idle: Duration) {
        let missing: Vec<_> = FILES.iter().filter(|f| !dir.join(f).is_file()).collect();
        if !missing.is_empty() {
            eprintln!("OCR disabled: missing {missing:?} in {}", dir.display());
            return;
        }
        let _ = SERVICE.set(Service {
            dir,
            idle,
            worker: Mutex::new(None),
            last_used: Mutex::new(Instant::now()),
            permit: Semaphore::new(1),
        });
        tokio::spawn(reaper());
        eprintln!(
            "OCR enabled (worker starts on first use, stops after {}s idle)",
            idle.as_secs()
        );
    }

    fn spawn_worker(dir: &Path) -> std::io::Result<Worker> {
        let mut child = Command::new(std::env::current_exe()?)
            .arg("ocr-worker")
            .arg(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().ok_or(std::io::ErrorKind::BrokenPipe)?;
        let stdout = BufReader::new(child.stdout.take().ok_or(std::io::ErrorKind::BrokenPipe)?);
        Ok(Worker {
            child,
            stdin,
            stdout,
        })
    }

    fn ask(w: &mut Worker, bytes: &[u8]) -> std::io::Result<String> {
        w.stdin.write_all(&(bytes.len() as u32).to_le_bytes())?;
        w.stdin.write_all(bytes)?;
        w.stdin.flush()?;
        let mut line = String::new();
        if w.stdout.read_line(&mut line)? == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        Ok(line)
    }

    pub async fn recognize(bytes: Bytes) -> Result<Vec<String>, (StatusCode, String)> {
        let svc = SERVICE.get().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "Image scanning is turned off on this server.".to_string(),
        ))?;
        let _permit = svc.permit.acquire().await.map_err(|_| internal("busy"))?;

        let reply = tokio::task::spawn_blocking(move || -> Result<String, (StatusCode, String)> {
            let mut slot = svc.worker.lock().unwrap();
            if slot.is_none() {
                *slot = Some(
                    spawn_worker(&svc.dir)
                        .map_err(|e| internal(format!("could not start OCR: {e}")))?,
                );
            }
            match ask(slot.as_mut().unwrap(), &bytes) {
                Ok(line) => Ok(line),
                Err(e) => {
                    // The worker died (crash, out of memory…): drop it so the
                    // next request starts a fresh one.
                    *slot = None;
                    Err(internal(format!("OCR stopped unexpectedly: {e}")))
                }
            }
        })
        .await
        .map_err(|e| internal(e.to_string()))??;

        *svc.last_used.lock().unwrap() = Instant::now();

        #[derive(serde::Deserialize)]
        struct Reply {
            #[serde(default)]
            lines: Vec<String>,
            error: Option<String>,
            status: Option<u16>,
        }
        let r: Reply = serde_json::from_str(&reply).map_err(|e| internal(e.to_string()))?;
        match r.error {
            None => Ok(r.lines),
            Some(e) => Err((
                StatusCode::from_u16(r.status.unwrap_or(500))
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                e,
            )),
        }
    }

    fn internal(msg: impl Into<String>) -> (StatusCode, String) {
        (StatusCode::INTERNAL_SERVER_ERROR, msg.into())
    }

    /// Stops the worker after it has been idle for a while.
    async fn reaper() {
        let Some(svc) = SERVICE.get() else { return };
        let mut tick = tokio::time::interval(Duration::from_secs(15));
        loop {
            tick.tick().await;
            let idle = svc.last_used.lock().unwrap().elapsed() >= svc.idle;
            // Never stop it in the middle of a request.
            if idle && let Ok(_p) = svc.permit.try_acquire() {
                if svc.worker.lock().unwrap().take().is_some() {
                    eprintln!("OCR worker stopped after being idle");
                }
            }
        }
    }

    // ------------------------------------------------------------ worker

    /// Entry point of `delphinus ocr-worker <model dir>`.
    pub fn worker_main(dir: &Path) -> ! {
        use oar_ocr::prelude::*;
        let started = Instant::now();
        let session = oar_ocr::core::config::OrtSessionConfig::new()
            .with_intra_threads(2)
            .with_inter_threads(1)
            .with_memory_pattern(false);
        let engine =
            match OAROCRBuilder::new(dir.join(FILES[0]), dir.join(FILES[1]), dir.join(FILES[2]))
                .ort_session(session)
                .image_batch_size(1)
                .region_batch_size(8)
                .build()
            {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("OCR worker: could not load models: {e}");
                    std::process::exit(1);
                }
            };
        eprintln!("OCR worker ready in {:?}", started.elapsed());

        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        let mut out = std::io::stdout().lock();
        loop {
            let mut len = [0u8; 4];
            if input.read_exact(&mut len).is_err() {
                std::process::exit(0); // server closed the pipe
            }
            let mut bytes = vec![0u8; u32::from_le_bytes(len) as usize];
            if input.read_exact(&mut bytes).is_err() {
                std::process::exit(0);
            }
            let reply = match decode(&bytes)
                .and_then(|img| engine.predict(vec![img]).map_err(|e| (500, e.to_string())))
            {
                Ok(results) => {
                    let mut lines = Vec::new();
                    for region in results.iter().flat_map(|r| &r.text_regions) {
                        if let Some((text, confidence)) = region.text_with_confidence() {
                            if confidence > 0.5 {
                                lines.extend(filter_cjk(text));
                            }
                        }
                    }
                    serde_json::json!({ "lines": lines })
                }
                Err((status, error)) => serde_json::json!({ "error": error, "status": status }),
            };
            drop(bytes);
            if writeln!(out, "{reply}").and_then(|_| out.flush()).is_err() {
                std::process::exit(0);
            }
        }
    }

    fn decode(bytes: &[u8]) -> Result<image::RgbImage, (u16, String)> {
        let bad = |m: &str| (422, m.to_string());
        let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| bad("That file is not an image."))?;
        let (w, h) = reader
            .into_dimensions()
            .map_err(|_| bad("Use a PNG, JPEG or WebP image."))?;
        if (w as u64) * (h as u64) > MAX_PIXELS {
            return Err(bad(
                "That image is too large. Crop it to the words you need.",
            ));
        }
        let img = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .ok()
            .and_then(|r| r.decode().ok())
            .ok_or_else(|| bad("Use a PNG, JPEG or WebP image."))?
            .into_rgb8();
        if img.width().max(img.height()) <= MAX_SIDE {
            return Ok(img);
        }
        let scale = MAX_SIDE as f32 / img.width().max(img.height()) as f32;
        let (nw, nh) = (
            (img.width() as f32 * scale) as u32,
            (img.height() as f32 * scale) as u32,
        );
        Ok(image::imageops::resize(
            &img,
            nw.max(1),
            nh.max(1),
            image::imageops::FilterType::Triangle,
        ))
    }

    /// Keeps CJK characters only, splitting at spaces and commas so each word
    /// becomes its own line. Kana inside parentheses is usually furigana and is
    /// dropped; kanji/hanzi are always kept.
    pub(super) fn filter_cjk(text: &str) -> Vec<String> {
        let mut inside_parens = false;
        let mut words = vec![String::new()];
        for c in text.chars() {
            let keep = match c {
                '(' | '（' => {
                    inside_parens = true;
                    false
                }
                ')' | '）' => {
                    inside_parens = false;
                    false
                }
                ' ' | '\u{3000}' | '、' | '，' | ',' | '；' | ';' | '。' => {
                    words.push(String::new());
                    false
                }
                '\u{4E00}'..='\u{9FFF}'
                | '\u{3400}'..='\u{4DBF}'
                | '\u{20000}'..='\u{2A6DF}'
                | '\u{2A700}'..='\u{2CEAF}'
                | '\u{F900}'..='\u{FAFF}'
                | '\u{2F800}'..='\u{2FA1F}'
                | '々' => true,
                '\u{3040}'..='\u{309F}'
                | '\u{30A0}'..='\u{30FF}'
                | '\u{31F0}'..='\u{31FF}'
                | '\u{FF66}'..='\u{FF9F}' => !inside_parens,
                _ => false,
            };
            if keep {
                words.last_mut().unwrap().push(c);
            }
        }
        words.retain(|w| !w.is_empty());
        words
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn filters_cjk() {
            assert_eq!(
                super::filter_cjk("漢字(かんじ) abc 図書館"),
                vec!["漢字", "図書館"]
            );
            assert_eq!(
                super::filter_cjk("写真　本棚、天気"),
                vec!["写真", "本棚", "天気"]
            );
            assert_eq!(super::filter_cjk("テレビ"), vec!["テレビ"]);
        }
    }
}

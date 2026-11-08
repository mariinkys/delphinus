#![recursion_limit = "256"]
#[cfg(feature = "ssr")]
fn main() -> std::io::Result<()> {
    // `delphinus ocr-worker`: the OCR child process (src/ocr.rs)
    #[cfg(feature = "ocr")]
    {
        let args: Vec<_> = std::env::args_os().collect();
        if args.get(1).is_some_and(|a| a == "ocr-worker") {
            let dir = args
                .get(2)
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| "ocr_models".into());
            delphinus::ocr::worker_main(&dir);
        }
    }

    // A small, fixed number of worker threads, more threads would only add stacks and allocator arenas
    let workers = env_parse("DELPHINUS_THREADS", 2usize).max(1);
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .max_blocking_threads(4)
        .thread_keep_alive(std::time::Duration::from_secs(10))
        .enable_all()
        .build()?
        .block_on(server::run())
}

#[cfg(feature = "ssr")]
fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(feature = "ssr")]
mod server {
    use super::env_parse;
    use axum::{
        Router,
        extract::DefaultBodyLimit,
        routing::{get, post},
    };
    use delphinus::{app::*, dict::Dictionaries, ocr};
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use std::path::PathBuf;

    /// Whether a file name carries a cargo-leptos content hash
    /// Only `cargo leptos build` adds hashes; `cargo leptos watch` serves
    /// plain `delphinus.css`, which must never be cached forever
    pub(crate) fn is_hashed(path: &str) -> bool {
        let name = path.rsplit('/').next().unwrap_or(path);
        name.split('.').skip(1).any(|part| {
            part.len() == 22
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
    }

    /// Hashed files may be kept forever: a new build gives them a new name
    async fn cache_headers(
        req: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        use axum::http::{HeaderValue, header::CACHE_CONTROL};
        let path = req.uri().path().to_owned();
        let mut res = next.run(req).await;
        if path.starts_with("/pkg/") && res.status().is_success() {
            let value = if is_hashed(&path) {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            res.headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static(value));
        }
        res
    }

    pub async fn run() -> std::io::Result<()> {
        let conf = get_configuration(None).expect("invalid Leptos configuration");
        let options = conf.leptos_options;
        let addr = options.site_addr;

        let path = |key: &str, default: &str| {
            PathBuf::from(std::env::var(key).unwrap_or_else(|_| default.into()))
        };
        let started = std::time::Instant::now();
        Dictionaries::load(
            &path("DELPHINUS_JMDICT", "dictionaries/JMdict_e"),
            &path("DELPHINUS_CEDICT", "dictionaries/cedict_ts.u8"),
        );
        eprintln!("dictionaries ready in {:?}", started.elapsed());

        tokio::spawn(async {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                tick.tick().await;
                if let Some(d) = Dictionaries::get() {
                    d.release_pages();
                }
            }
        });

        let ocr_off = std::env::var("DISABLE_OCR").is_ok_and(|v| v.eq_ignore_ascii_case("true"));
        #[cfg(feature = "ocr")]
        if !ocr_off {
            ocr::init(
                path("DELPHINUS_OCR_DIR", "ocr_models"),
                std::time::Duration::from_secs(env_parse("DELPHINUS_OCR_IDLE_SECS", 300u64)),
            );
        }
        let _ = (ocr_off, env_parse::<u8>);

        let routes = generate_route_list(App);
        let app = Router::new()
            .route(
                "/api/ocr",
                post(ocr::handler).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
            )
            .route("/healthz", get(|| async { "ok" }))
            .route(
                "/faq",
                get(|| async { axum::response::Redirect::permanent("/guide") }),
            )
            .route(
                "/generator",
                get(|| async { axum::response::Redirect::permanent("/") }),
            )
            .leptos_routes(&options, routes, {
                let options = options.clone();
                move || shell(options.clone())
            })
            .fallback(leptos_axum::file_and_error_handler(shell))
            .layer(axum::middleware::from_fn(cache_headers))
            .with_state(options);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        eprintln!("listening on http://{addr}");
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async {
                #[cfg(unix)]
                {
                    use tokio::signal::unix::{SignalKind, signal};
                    let mut term = signal(SignalKind::terminate()).expect("signal handler");
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => {},
                        _ = term.recv() => {},
                    }
                }
                #[cfg(not(unix))]
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // The browser entry point is `hydrate()` in lib.rs.
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::server::is_hashed;

    #[test]
    fn recognises_hashed_assets() {
        assert!(is_hashed("/pkg/delphinus.uPi5MofTdzMX4W5T8WrfPQ.css"));
        assert!(is_hashed("/pkg/delphinus.o8g-JOWX_XBGhMFDf9b-Hg.wasm"));
        assert!(
            !is_hashed("/pkg/delphinus.css"),
            "cargo leptos watch output"
        );
        assert!(!is_hashed("/pkg/delphinus_bg.wasm"));
        assert!(!is_hashed("/pkg/delphinus.js"));
    }
}

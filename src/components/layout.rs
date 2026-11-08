use leptos::prelude::*;
use leptos_router::components::A;
use std::time::Duration;

#[component]
pub fn SiteHeader() -> impl IntoView {
    view! {
        <header class="site-header">
            <A href="/" attr:class="wordmark">
                <span class="wordmark-name">"Delphinus"</span>
            </A>
            <nav class="site-nav" aria-label="Main">
                <A href="/" exact=true>"Make cards"</A>
                <A href="/guide">"Guide"</A>
                <ThemeToggle />
            </nav>
        </header>
    }
}

#[component]
fn ThemeToggle() -> impl IntoView {
    let dark = RwSignal::new(false);
    Effect::new(move |_| dark.set(crate::client::is_dark()));
    view! {
        <button
            class="theme-toggle"
            type="button"
            aria-label=move || if dark.get() { "Switch to light theme" } else { "Switch to dark theme" }
            on:click=move |_| {
                let next = !crate::client::is_dark();
                crate::client::set_theme(next);
                dark.set(next);
            }
        >
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
                <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="1.6" />
                <path d="M12 3.5a8.5 8.5 0 0 1 0 17z" fill="currentColor" />
            </svg>
        </button>
    }
}

/// A short status message, oone at a time.
#[derive(Clone, Copy)]
pub struct Notice(RwSignal<Option<(String, bool, u64)>>);

impl Notice {
    pub fn provide() -> Self {
        let n = Notice(RwSignal::new(None));
        provide_context(n);
        n
    }
    pub fn use_() -> Self {
        expect_context()
    }
    pub fn info(self, msg: impl Into<String>) {
        self.show(msg.into(), false)
    }
    pub fn error(self, msg: impl Into<String>) {
        self.show(msg.into(), true)
    }
    fn show(self, msg: String, error: bool) {
        let id = self.0.with_untracked(|n| n.as_ref().map_or(0, |n| n.2 + 1));
        self.0.set(Some((msg, error, id)));
        let sig = self.0;
        set_timeout(
            move || {
                if sig.with_untracked(|n| n.as_ref().is_some_and(|n| n.2 == id)) {
                    sig.set(None);
                }
            },
            Duration::from_secs(if error { 8 } else { 4 }),
        );
    }
}

#[component]
pub fn NoticeArea() -> impl IntoView {
    let Notice(n) = Notice::use_();
    view! {
        <div class="notice-area" role="status" aria-live="polite">
            {move || {
                n.get()
                    .map(|(msg, error, _)| {
                        view! { <p class="notice" class:is-error=error>{msg}</p> }
                    })
            }}
        </div>
    }
}

#[component]
pub fn SiteFooter() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <p>
                "Made by Alex Marín. Free and open source, now and always. "
                <a href="https://github.com/mariinkys/delphinus/releases">"Version " {env!("CARGO_PKG_VERSION")}</a>
            </p>
            <p class="credits">
                "Japanese from "
                <a href="https://www.edrdg.org/wiki/index.php/JMdict-EDICT_Dictionary_Project">"JMdict"</a>
                ", property of the "
                <a href="https://www.edrdg.org/">"EDRDG"</a>
                ", used under its "
                <a href="https://www.edrdg.org/edrdg/licence.html">"licence"</a>
                ". Chinese from "
                <a href="https://cc-cedict.org/wiki/">"CC-CEDICT"</a>
                " (CC BY-SA 4.0)."
            </p>
            <nav class="footer-links" aria-label="Elsewhere">
                <a href="https://github.com/mariinkys/delphinus">"Source code"</a>
                <a href="https://www.buymeacoffee.com/mariinkys">"Buy me a coffee"</a>
                <a href="https://mastodon.social/@mariinkys" rel="me">"Mastodon"</a>
            </nav>
        </footer>
    }
}

use crate::{
    components::layout::{SiteFooter, SiteHeader},
    pages::{Guide, NotFound, Workspace},
};
use leptos::prelude::*;
use leptos_meta::{HashedStylesheet, Link, Meta, MetaTags, Title, provide_meta_context};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

/// Applies the saved theme before first paint, so dark mode never flashes
const THEME_SCRIPT: &str = "try{var t=localStorage.getItem('delphinus.theme');if(t)document.documentElement.setAttribute('data-theme',t)}catch(e){}";

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="color-scheme" content="light dark" />
                <script inner_html=THEME_SCRIPT></script>
                <HashedStylesheet options=options.clone() id="leptos" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title formatter=|t: String| if t.is_empty() { "Delphinus".to_string() } else { format!("{t} | Delphinus") } />
        <Meta name="description" content="Turn a list of Chinese or Japanese words into flashcards for Anki, Quizlet or Vaia, with readings and meanings from CC-CEDICT and JMdict." />
        <Meta name="theme-color" content="#e9efe9" />
        <Link rel="icon" type_="image/png" sizes="32x32" href="/favicon-32x32.png" />
        <Link rel="icon" type_="image/png" sizes="16x16" href="/favicon-16x16.png" />
        <Link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png" />
        <Link rel="manifest" href="/site.webmanifest" />

        <Router>
            <a class="skip" href="#main">"Skip to content"</a>
            <SiteHeader />
            <main id="main">
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("") view=Workspace />
                    <Route path=path!("/guide") view=Guide />
                </Routes>
            </main>
            <SiteFooter />
        </Router>
    }
}

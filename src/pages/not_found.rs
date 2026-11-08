use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

#[component]
pub fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
        resp.set_status(axum::http::StatusCode::NOT_FOUND);
    }
    view! {
        <Title text="Page not found" />
        <div class="not-found">
            <p class="not-found-cells" aria-hidden="true">
                <span>"无"</span>
                <span>"い"</span>
            </p>
            <h1>"There is no page at this address"</h1>
            <p>"The link may be old or mistyped."</p>
            <A href="/" attr:class="button button-primary">"Make flashcards"</A>
        </div>
    }
}

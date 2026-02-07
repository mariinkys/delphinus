use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use leptos::web_sys::window;

#[component]
pub fn NavbarComponent() -> impl IntoView {
    let dark_mode = RwSignal::new(false);

    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        #[allow(clippy::collapsible_if)]
        if let Some(stored_theme) = get_stored_theme() {
            dark_mode.set(stored_theme);
        } else {
            if let Some(win) = window() {
                if let Ok(media_query) = win.match_media("(prefers-color-scheme: dark)") {
                    #[allow(clippy::collapsible_match)]
                    if let Some(mq) = media_query {
                        dark_mode.set(mq.matches());
                    }
                }
            }
        };
    });

    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        let theme = if dark_mode.get() { "dark" } else { "light" };

        #[cfg(target_arch = "wasm32")]
        {
            // apply theme
            if let Some(document) = web_sys::window().unwrap().document()
                && let Some(html) = document.document_element()
            {
                html.set_attribute("data-theme", theme).unwrap();
            }

            // save to localStorage
            save_theme_preference(dark_mode.get());
        }
    });

    view! {
        <div class="navbar bg-base-100">
            <div class="navbar-start">
                <div class="dropdown">
                    <div tabindex="0" role="button" class="btn btn-ghost lg:hidden">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h8m-8 6h16" /></svg>
                    </div>
                    <ul tabindex="0" class="menu menu-sm dropdown-content mt-3 z-1 p-2 shadow bg-base-100 rounded-box w-52">
                        <li><a href="/generator">"Generate Flashcards"</a></li>
                        <li><a href="/faq">"FAQ"</a></li>
                    </ul>
                </div>
                <a class="btn btn-ghost text-xl" href="/">"Delphinus"</a>
            </div>
            <div class="navbar-end gap-2 flex items-center">
                <label class="swap swap-rotate">
                    <input type="checkbox" prop:checked=move || dark_mode.get()
                        on:input=move |_| dark_mode.update(|v| *v = !*v)
                     />
                    <svg class="swap-on fill-current w-10 h-10" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M5.64,17l-.71.71a1,1,0,0,0,0,1.41,1,1,0,0,0,1.41,0l.71-.71A1,1,0,0,0,5.64,17ZM5,12a1,1,0,0,0-1-1H3a1,1,0,0,0,0,2H4A1,1,0,0,0,5,12Zm7-7a1,1,0,0,0,1-1V3a1,1,0,0,0-2,0V4A1,1,0,0,0,12,5ZM5.64,7.05a1,1,0,0,0,.7.29,1,1,0,0,0,.71-.29,1,1,0,0,0,0-1.41l-.71-.71A1,1,0,0,0,4.93,6.34Zm12,.29a1,1,0,0,0,.7-.29l.71-.71a1,1,0,1,0-1.41-1.41L17,5.64a1,1,0,0,0,0,1.41A1,1,0,0,0,17.66,7.34ZM21,11H20a1,1,0,0,0,0,2h1a1,1,0,0,0,0-2Zm-9,8a1,1,0,0,0-1,1v1a1,1,0,0,0,2,0V20A1,1,0,0,0,12,19ZM18.36,17A1,1,0,0,0,17,18.36l.71.71a1,1,0,0,0,1.41,0,1,1,0,0,0,0-1.41ZM12,6.5A5.5,5.5,0,1,0,17.5,12,5.51,5.51,0,0,0,12,6.5Zm0,9A3.5,3.5,0,1,1,15.5,12,3.5,3.5,0,0,1,12,15.5Z"/></svg>
                    <svg class="swap-off fill-current w-10 h-10" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M21.64,13a1,1,0,0,0-1.05-.14,8.05,8.05,0,0,1-3.37.73A8.15,8.15,0,0,1,9.08,5.49a8.59,8.59,0,0,1,.25-2A1,1,0,0,0,8,2.36,10.14,10.14,0,1,0,22,14.05,1,1,0,0,0,21.64,13Zm-9.5,6.69A8.14,8.14,0,0,1,7.08,5.22v.27A10.15,10.15,0,0,0,17.22,15.63a9.79,9.79,0,0,0,2.1-.22A8.11,8.11,0,0,1,12.14,19.73Z"/></svg>
                </label>
                <a class="btn btn-primary hidden sm:flex" href="/faq">"FAQ"</a>
            </div>
        </div>
    }.into_any()
}

#[cfg(target_arch = "wasm32")]
fn get_local_storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok().flatten()
}

// Get stored theme preference
#[cfg(target_arch = "wasm32")]
fn get_stored_theme() -> Option<bool> {
    let storage = get_local_storage()?;
    let theme = storage.get_item("theme").ok().flatten()?;
    match theme.as_str() {
        "dark" => Some(true),
        "light" => Some(false),
        _ => None,
    }
}

// Save theme preference
#[cfg(target_arch = "wasm32")]
fn save_theme_preference(is_dark: bool) {
    if let Some(storage) = get_local_storage() {
        let theme = if is_dark { "dark" } else { "light" };
        let _ = storage.set_item("theme", theme);
    }
}

use crate::{
    api::{get_status, lookup_words},
    client::{storage_get, storage_set},
    components::{
        cards::{CardList, Deck},
        export::ExportBar,
        layout::{Notice, NoticeArea},
        options::CardOptions,
        scan::{ScanButton, scan_file},
    },
    model::{CardFormat, Lang, MAX_TERMS, split_terms, to_full_width},
};
use leptos::{prelude::*, server_fn::error::ServerFnError, task::spawn_local};
use leptos_meta::Title;
use wasm_bindgen::JsCast;

const KEY_INPUT: &str = "delphinus.input";
const KEY_LANG: &str = "delphinus.lang";
const KEY_FORMAT: &str = "delphinus.format";

fn placeholder(lang: Lang) -> &'static str {
    match lang {
        Lang::Zh => "图书馆\n复习，中文\n气温",
        Lang::Ja => "図書館\n写真、本棚\n空",
    }
}

/// Rewrites the word box in full-width characters so every character fills
/// exactly one grid cell, keeping the caret where it was. Each converted
/// character is one UTF-16 unit before and after, so offsets stay valid;
/// half-width katakana with a separate (han)dakuten can shrink by one, so the
/// caret is then moved back by the difference
fn fit_to_grid(el: &web_sys::HtmlTextAreaElement) -> String {
    let raw = el.value();
    let wide = to_full_width(&raw);
    if wide != raw {
        let shrink = raw.encode_utf16().count() as u32 - wide.encode_utf16().count() as u32;
        let start = el.selection_start().ok().flatten();
        let end = el.selection_end().ok().flatten();
        el.set_value(&wide);
        if let (Some(s), Some(e)) = (start, end) {
            let _ = el.set_selection_range(s.saturating_sub(shrink), e.saturating_sub(shrink));
        }
    }
    wide.to_string()
}

fn error_text(e: ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => m,
        _ => "Couldn't reach the server. Check your connection and try again.".to_string(),
    }
}

#[component]
pub fn Workspace() -> impl IntoView {
    let notice = Notice::provide();
    let status = OnceResource::new(get_status());
    let ocr_on = Signal::derive(move || status.get().and_then(|r| r.ok()).is_some_and(|s| s.ocr));

    let lang = RwSignal::new(Lang::Zh);
    let input = RwSignal::new(String::new());
    let fmt = RwSignal::new(CardFormat::default());
    let deck = Deck::new();
    let busy = RwSignal::new(false);
    let scanning = RwSignal::new(false);
    let terms = Memo::new(move |_| split_terms(&input.read()));

    // Restore the last session in this browser, then keep it saved
    Effect::new(move |_| {
        if let Some(v) = storage_get(KEY_INPUT) {
            input.set(to_full_width(&v));
        }
        if let Some(l) = storage_get(KEY_LANG).and_then(|v| Lang::from_code(&v)) {
            lang.set(l);
        }
        if let Some(f) = storage_get(KEY_FORMAT).and_then(|v| serde_json::from_str(&v).ok()) {
            fmt.set(f);
        }
    });
    Effect::new(move |prev: Option<()>| {
        let (i, l, f) = (input.get(), lang.get(), fmt.get());
        if prev.is_some() {
            storage_set(KEY_INPUT, &i);
            storage_set(KEY_LANG, l.code());
            storage_set(KEY_FORMAT, &serde_json::to_string(&f).unwrap_or_default());
        }
    });
    // Formatting changes apply to every card the person has not edited
    Effect::new(move |prev: Option<()>| {
        let f = fmt.get();
        if prev.is_some() {
            deck.reformat(&f);
        }
    });

    let look_up = move || {
        let t = terms.get_untracked();
        if t.is_empty() {
            notice.error("Write at least one word first.");
            return;
        }
        if busy.get_untracked() {
            return;
        }
        let l = lang.get_untracked();
        busy.set(true);
        spawn_local(async move {
            match lookup_words(l, t).await {
                Ok(found) => {
                    deck.fill(found, l, &fmt.get_untracked());
                    request_animation_frame(|| {
                        if let Some(el) = web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.get_element_by_id("deck-title"))
                        {
                            el.scroll_into_view();
                        }
                    });
                }
                Err(e) => notice.error(error_text(e)),
            }
            busy.set(false);
        });
    };

    let add_lines = Callback::new(move |lines: Vec<String>| {
        input.update(|s| {
            if !s.trim().is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(&to_full_width(&lines.join("\n")));
        })
    });

    // Pasting or dropping an image onto the paper scans it
    let take_image = move |files: Option<web_sys::FileList>| -> bool {
        let Some(file) = files.and_then(|f| f.get(0)) else {
            return false;
        };
        if !file.type_().starts_with("image/") {
            return false;
        }
        if ocr_on.get_untracked() {
            scan_file(file, scanning, add_lines, notice);
        } else {
            notice.error("Image scanning is not available on this server.");
        }
        true
    };

    view! {
        <Title text="" />
        <div class="workspace">
            <section class="compose" aria-labelledby="compose-title">
                <h1 id="compose-title">"Make flashcards from a list of words"</h1>
                <p class="lede">
                    "Write one word per line, or separate them with commas. Delphinus adds the reading and meaning of each, ready for Anki, Quizlet or Vaia."
                </p>

                <fieldset class="lang-switch">
                    <legend class="visually-hidden">"Language"</legend>
                    {[Lang::Zh, Lang::Ja]
                        .into_iter()
                        .map(|l| {
                            view! {
                                <label class="lang-option">
                                    <input
                                        type="radio"
                                        name="lang"
                                        prop:checked=move || lang.get() == l
                                        on:change=move |_| lang.set(l)
                                    />
                                    <span class="lang-native" lang=l.code()>{l.native_name()}</span>
                                    <span class="lang-english">{l.english_name()}</span>
                                </label>
                            }
                        })
                        .collect_view()}
                </fieldset>

                <label for="words" class="visually-hidden">"Words"</label>
                <textarea
                    id="words"
                    class="paper"
                    lang=move || lang.get().code()
                    spellcheck="false"
                    autocomplete="off"
                    placeholder=move || placeholder(lang.get())
                    prop:value=input
                    on:input=move |ev| {
                        // Leave the text alone while an IME is composing
                        // (romaji typed for kana); compositionend handles it
                        if ev.dyn_ref::<web_sys::InputEvent>().is_some_and(|e| e.is_composing()) {
                            input.set(event_target_value(&ev));
                        } else {
                            input.set(fit_to_grid(&event_target(&ev)));
                        }
                    }
                    on:compositionend=move |ev| input.set(fit_to_grid(&event_target(&ev)))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" && (ev.ctrl_key() || ev.meta_key()) {
                            ev.prevent_default();
                            look_up();
                        }
                    }
                    on:paste=move |ev| {
                        if take_image(ev.clipboard_data().and_then(|d| d.files())) {
                            ev.prevent_default();
                        }
                    }
                    on:dragover=move |ev| ev.prevent_default()
                    on:drop=move |ev| {
                        let files = ev
                            .data_transfer()
                            .and_then(|d| d.files());
                        if take_image(files) {
                            ev.prevent_default();
                        }
                    }
                ></textarea>

                <div class="compose-actions">
                    <p class="term-count" aria-live="polite">
                        {move || {
                            let n = terms.read().len();
                            match n {
                                0 => "No words yet".to_string(),
                                1 => "1 word".to_string(),
                                n if n >= MAX_TERMS => format!("{n} words (the maximum per lookup)"),
                                n => format!("{n} words"),
                            }
                        }}
                    </p>
                    <Suspense fallback=|| ()>
                        {move || ocr_on.get().then(|| view! { <ScanButton busy=scanning on_lines=add_lines /> })}
                    </Suspense>
                    <button
                        type="button"
                        class="button button-primary"
                        disabled=busy
                        on:click=move |_| look_up()
                        title="Ctrl + Enter"
                    >
                        {move || if busy.get() { "Looking up…" } else { "Look up words" }}
                    </button>
                </div>
            </section>

            <CardOptions fmt lang />
        </div>

        <Show when=move || !deck.groups.read().is_empty()>
            <CardList deck fmt />
            <ExportBar deck />
        </Show>
        <NoticeArea />
    }
}

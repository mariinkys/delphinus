use super::{cards::Deck, layout::Notice};
use crate::{
    client::{storage_get, storage_set},
    model::{CustomFormat, ExportTarget, export, export_custom},
};
use leptos::{html, prelude::*, task::spawn_local};

const KEY_CUSTOM: &str = "delphinus.custom";

/// Copies `text` to the clipboard (or downloads it as `file_name` when the
/// clipboard is unavailable) and reports how it went
fn copy_or_download(
    notice: Notice,
    text: String,
    n: usize,
    what: &'static str,
    file_name: &'static str,
) {
    let plural = if n == 1 { "card" } else { "cards" };
    spawn_local(async move {
        if crate::client::copy_text(&text).await {
            notice.info(format!("Copied {n} {plural} {what}."));
        } else {
            crate::client::download(file_name, "text/plain;charset=utf-8", &text);
            notice.error(format!(
                "Couldn't use the clipboard, so the text was downloaded as {file_name}."
            ));
        }
    });
}

fn download_file(notice: Notice, text: &str, n: usize, file_name: &str, mime: &str) {
    let plural = if n == 1 { "card" } else { "cards" };
    if crate::client::download(file_name, mime, text) {
        notice.info(format!("Downloaded {n} {plural} as {file_name}."));
    } else {
        notice.error("Your browser blocked the download.");
    }
}

/// Sticky bar with every way to take the cards elsewhere
#[component]
pub fn ExportBar(deck: Deck) -> impl IntoView {
    let notice = Notice::use_();
    let count = Memo::new(move |_| deck.selected().len());
    let dialog = NodeRef::<html::Dialog>::new();

    let run = move |target: ExportTarget| {
        let cards = deck.selected();
        if cards.is_empty() {
            notice.error("Select at least one card first.");
            return;
        }
        let text = export(target, &cards);
        match target {
            ExportTarget::Anki | ExportTarget::Csv => download_file(
                notice,
                &text,
                cards.len(),
                target.file_name(),
                target.mime(),
            ),
            ExportTarget::Quizlet => copy_or_download(
                notice,
                text,
                cards.len(),
                "for Quizlet. Paste them into its import box",
                "delphinus_quizlet.txt",
            ),
            ExportTarget::Vaia => copy_or_download(
                notice,
                text,
                cards.len(),
                "for Vaia. Paste them into its import box",
                "delphinus_vaia.txt",
            ),
        }
    };

    view! {
        <div class="export-bar" role="region" aria-label="Export">
            <p class="export-count">
                <strong>{count}</strong>
                {move || if count.get() == 1 { " card ready" } else { " cards ready" }}
            </p>
            <div class="export-actions">
                {ExportTarget::ALL
                    .into_iter()
                    .map(|t| {
                        let label = match t {
                            ExportTarget::Anki => "Download for Anki",
                            ExportTarget::Quizlet => "Copy for Quizlet",
                            ExportTarget::Vaia => "Copy for Vaia",
                            ExportTarget::Csv => "Download CSV",
                        };
                        view! {
                            <button
                                type="button"
                                class="button"
                                class:button-primary=t == ExportTarget::Anki
                                disabled=move || count.get() == 0
                                on:click=move |_| run(t)
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect_view()}
                <button
                    type="button"
                    class="button"
                    aria-haspopup="dialog"
                    disabled=move || count.get() == 0
                    on:click=move |_| {
                        if let Some(d) = dialog.get() {
                            let _ = d.show_modal();
                        }
                    }
                >
                    "Custom format"
                </button>
            </div>
            <CustomExportDialog dialog deck notice />
        </div>
    }
}

/// A modal where the person picks what goes between the front and back of a card and between cards
#[component]
fn CustomExportDialog(dialog: NodeRef<html::Dialog>, deck: Deck, notice: Notice) -> impl IntoView {
    let fmt = RwSignal::new(CustomFormat::default());
    let open = RwSignal::new(false);

    Effect::new(move |_| {
        if let Some(f) = storage_get(KEY_CUSTOM).and_then(|v| serde_json::from_str(&v).ok()) {
            fmt.set(f);
        }
    });
    Effect::new(move |prev: Option<()>| {
        let f = fmt.get();
        if prev.is_some() {
            storage_set(KEY_CUSTOM, &serde_json::to_string(&f).unwrap_or_default());
        }
    });

    let close = move || {
        if let Some(d) = dialog.get_untracked() {
            d.close();
        }
    };

    let preview = move || {
        if !open.get() {
            return String::new();
        }
        let sample: Vec<_> = deck.selected().into_iter().take(3).collect();
        export_custom(&sample, &fmt.get())
    };

    let cards_or_warn = move || {
        let cards = deck.selected();
        if cards.is_empty() {
            notice.error("Select at least one card first.");
            None
        } else {
            Some(cards)
        }
    };
    // Each action closes the modal first so its confirmation is visible
    let copy = move |_| {
        if let Some(cards) = cards_or_warn() {
            let text = export_custom(&cards, &fmt.get_untracked());
            close();
            copy_or_download(
                notice,
                text,
                cards.len(),
                "in your format",
                "delphinus_custom.txt",
            );
        }
    };
    let download = move |_| {
        if let Some(cards) = cards_or_warn() {
            let text = export_custom(&cards, &fmt.get_untracked());
            close();
            download_file(
                notice,
                &text,
                cards.len(),
                "delphinus_custom.txt",
                "text/plain;charset=utf-8",
            );
        }
    };

    view! {
        <dialog
            node_ref=dialog
            class="modal"
            aria-labelledby="custom-title"
            // `toggle` fires on open and close, however they happen
            on:toggle=move |_| open.set(dialog.get_untracked().is_some_and(|d| d.open()))
            // A click on the dialog element itself is a click on the backdrop:
            // the visible box is the inner .modal-body
            on:click=move |ev| {
                if let Some(d) = dialog.get_untracked()
                    && ev.target().as_ref() == Some(d.as_ref())
                {
                    d.close();
                }
            }
        >
            <div class="modal-body">
                <header class="modal-head">
                    <h2 id="custom-title">"Custom format"</h2>
                    <button type="button" class="modal-close" aria-label="Close" on:click=move |_| close()>
                        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
                            <path d="M6 6l12 12M18 6L6 18" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
                        </svg>
                    </button>
                </header>
                <p class="modal-text">
                    "Choose what goes between the front and back of each card, and between cards. Type "
                    <code>"\\t"</code> " for a tab and " <code>"\\n"</code> " for a new line."
                </p>
                <div class="modal-fields">
                    <label class="field">
                        <span class="field-label">"Between front and back"</span>
                        <input
                            type="text"
                            spellcheck="false"
                            autocomplete="off"
                            prop:value=move || fmt.get().between_sides
                            on:input=move |ev| fmt.update(|f| f.between_sides = event_target_value(&ev))
                        />
                    </label>
                    <label class="field">
                        <span class="field-label">"Between cards"</span>
                        <input
                            type="text"
                            spellcheck="false"
                            autocomplete="off"
                            prop:value=move || fmt.get().between_cards
                            on:input=move |ev| fmt.update(|f| f.between_cards = event_target_value(&ev))
                        />
                    </label>
                </div>
                <div class="field">
                    <span class="field-label" id="custom-preview-label">"Preview of the first cards"</span>
                    <pre class="modal-preview" aria-labelledby="custom-preview-label">{preview}</pre>
                </div>
                <div class="modal-actions">
                    <button type="button" class="button" on:click=move |_| close()>
                        "Cancel"
                    </button>
                    <button type="button" class="button" on:click=download>
                        "Download"
                    </button>
                    <button type="button" class="button button-primary" on:click=copy>
                        "Copy"
                    </button>
                </div>
            </div>
        </dialog>
    }
}

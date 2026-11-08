use super::{cards::Deck, layout::Notice};
use crate::model::{ExportTarget, export};
use leptos::{prelude::*, task::spawn_local};

/// Sticky bar with every way to take the cards elsewhere
#[component]
pub fn ExportBar(deck: Deck) -> impl IntoView {
    let notice = Notice::use_();
    let count = Memo::new(move |_| deck.selected().len());

    let run = move |target: ExportTarget| {
        let cards = deck.selected();
        if cards.is_empty() {
            notice.error("Select at least one card first.");
            return;
        }
        let n = cards.len();
        let plural = if n == 1 { "card" } else { "cards" };
        let text = export(target, &cards);
        if target.is_file() {
            if crate::client::download(target.file_name(), target.mime(), &text) {
                notice.info(format!(
                    "Downloaded {n} {plural} as {}.",
                    target.file_name()
                ));
            } else {
                notice.error("Your browser blocked the download.");
            }
            return;
        }
        spawn_local(async move {
            if crate::client::copy_text(&text).await {
                notice.info(format!(
                    "Copied {n} {plural} for {}. Paste them into its import box.",
                    target.name()
                ));
            } else {
                let name = format!("delphinus_{}.txt", target.name().to_lowercase());
                crate::client::download(&name, "text/plain;charset=utf-8", &text);
                notice.error(format!(
                    "Couldn't use the clipboard, so the {} text was downloaded as {name}.",
                    target.name()
                ));
            }
        });
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
            </div>
        </div>
    }
}

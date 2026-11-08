//! The list of looked-up words and their editable cards.

use crate::model::{
    Candidate, CardFormat, Lang, Lookup, card_back, card_front, display_reading, tag_label,
};
use leptos::prelude::*;

/// One possible flashcard
#[derive(Clone)]
pub struct Row {
    pub id: usize,
    pub cand: Option<Candidate>,
    pub include: RwSignal<bool>,
    pub front: RwSignal<String>,
    pub back: RwSignal<String>,
    pub edited: RwSignal<bool>,
}

/// Everything found for one searched word
#[derive(Clone)]
pub struct Group {
    pub id: usize,
    pub term: String,
    pub rows: Vec<Row>,
}

impl Group {
    pub fn found(&self) -> bool {
        self.rows.iter().any(|r| r.cand.is_some())
    }
}

#[derive(Clone, Copy)]
pub struct Deck {
    pub groups: RwSignal<Vec<Group>>,
    pub lang: RwSignal<Lang>,
    /// Ids are never reused across lookups, so `<For>` re-renders new results
    next_id: StoredValue<usize>,
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

impl Deck {
    pub fn new() -> Self {
        Self {
            groups: RwSignal::new(Vec::new()),
            lang: RwSignal::new(Lang::Zh),
            next_id: StoredValue::new(0),
        }
    }

    pub fn fill(&self, lookups: Vec<Lookup>, lang: Lang, fmt: &CardFormat) {
        let mut next_id = self.next_id.get_value();
        let groups = lookups
            .into_iter()
            .map(|l| {
                next_id += 1;
                let gid = next_id;
                let rows = if l.candidates.is_empty() {
                    next_id += 1;
                    vec![Row {
                        id: next_id,
                        cand: None,
                        include: RwSignal::new(false),
                        front: RwSignal::new(l.term.clone()),
                        back: RwSignal::new(String::new()),
                        edited: RwSignal::new(false),
                    }]
                } else {
                    l.candidates
                        .into_iter()
                        .enumerate()
                        .map(|(i, c)| {
                            next_id += 1;
                            Row {
                                id: next_id,
                                include: RwSignal::new(i == 0),
                                front: RwSignal::new(card_front(&c, lang, fmt)),
                                back: RwSignal::new(card_back(&c, lang, fmt)),
                                edited: RwSignal::new(false),
                                cand: Some(c),
                            }
                        })
                        .collect()
                };
                Group {
                    id: gid,
                    term: l.term,
                    rows,
                }
            })
            .collect();
        self.next_id.set_value(next_id);
        self.lang.set(lang);
        self.groups.set(groups);
    }

    /// Re-renders every card the person has not edited
    pub fn reformat(&self, fmt: &CardFormat) {
        let lang = self.lang.get_untracked();
        self.groups.with_untracked(|gs| {
            for row in gs.iter().flat_map(|g| &g.rows) {
                if let (false, Some(c)) = (row.edited.get_untracked(), &row.cand) {
                    row.front.set(card_front(c, lang, fmt));
                    row.back.set(card_back(c, lang, fmt));
                }
            }
        });
    }

    pub fn selected(&self) -> Vec<(String, String)> {
        self.groups.with(|gs| {
            gs.iter()
                .flat_map(|g| &g.rows)
                .filter(|r| r.include.get())
                .map(|r| (r.front.get(), r.back.get()))
                .filter(|(f, b)| !f.trim().is_empty() || !b.trim().is_empty())
                .collect()
        })
    }

    pub fn select_all_matches(&self, on: bool) {
        self.groups.with_untracked(|gs| {
            for g in gs {
                for (i, r) in g.rows.iter().enumerate() {
                    if r.cand.is_some() {
                        r.include.set(if on { true } else { i == 0 });
                    }
                }
            }
        });
    }
}

#[component]
pub fn CardList(deck: Deck, fmt: RwSignal<CardFormat>) -> impl IntoView {
    let found = Memo::new(move |_| {
        deck.groups
            .with(|gs| gs.iter().filter(|g| g.found()).count())
    });
    let total = Memo::new(move |_| deck.groups.with(|gs| gs.len()));
    let selected = Memo::new(move |_| {
        deck.groups.with(|gs| {
            gs.iter()
                .flat_map(|g| &g.rows)
                .filter(|r| r.include.get())
                .count()
        })
    });
    let all_on = RwSignal::new(false);

    view! {
        <section class="deck" aria-labelledby="deck-title">
            <header class="deck-head">
                <h2 id="deck-title">"Your cards"</h2>
                <p class="deck-summary">
                    {move || {
                        let (f, t, s) = (found.get(), total.get(), selected.get());
                        let missing = if f < t {
                            format!(" {} not in the dictionary.", t - f)
                        } else {
                            String::new()
                        };
                        format!("Found {f} of {t} words.{missing} {s} {} selected.", if s == 1 { "card" } else { "cards" })
                    }}
                </p>
                <label class="switch">
                    <input
                        type="checkbox"
                        prop:checked=all_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            all_on.set(on);
                            deck.select_all_matches(on);
                        }
                    />
                    " Include every match, not just the first"
                </label>
            </header>
            <ol class="entries">
                <For each=move || deck.groups.get() key=|g| g.id let:group>
                    <Entry group lang=deck.lang fmt />
                </For>
            </ol>
        </section>
    }
}

#[component]
fn Entry(group: Group, lang: RwSignal<Lang>, fmt: RwSignal<CardFormat>) -> impl IntoView {
    let found = group.found();
    let count = group.rows.len();
    let term = group.term.clone();
    view! {
        <li class="entry" class:is-missing=!found>
            <div class="entry-term" lang=move || lang.get().code()>
                <span class="term">{term}</span>
                {(found && count > 1).then(|| view! { <span class="entry-count">{count} " matches"</span> })}
            </div>
            <div class="entry-rows">
                {(!found)
                    .then(|| {
                        view! {
                            <p class="entry-missing">
                                "Not in the dictionary. Check the spelling, or write the back yourself to keep it."
                            </p>
                        }
                    })}
                {group.rows.into_iter().map(|row| view! { <CardRow row lang fmt /> }).collect_view()}
            </div>
        </li>
    }
}

#[component]
fn CardRow(row: Row, lang: RwSignal<Lang>, fmt: RwSignal<CardFormat>) -> impl IntoView {
    let Row {
        id,
        cand,
        include,
        front,
        back,
        edited,
    } = row;
    let has_cand = cand.is_some();
    let front_id = format!("front-{id}");
    let back_id = format!("back-{id}");
    let detail = cand.clone().map(|c| {
        let reading = move || display_reading(&c, lang.get(), &fmt.get());
        let tags: Vec<String> = {
            let mut t: Vec<String> = Vec::new();
            for s in cand.as_ref().into_iter().flat_map(|c| &c.senses) {
                for tag in &s.tags {
                    let l = tag_label(tag).to_string();
                    if !t.contains(&l) {
                        t.push(l);
                    }
                }
            }
            t.truncate(3);
            t
        };
        let common = cand.as_ref().is_some_and(|c| c.common);
        view! {
            <p class="row-meta">
                <span class="row-reading" lang=move || lang.get().code()>{reading}</span>
                {common.then(|| view! { <span class="pill">"common"</span> })}
                {tags.into_iter().map(|t| view! { <span class="pill pill-quiet">{t}</span> }).collect_view()}
            </p>
        }
    });
    let stored = StoredValue::new(cand);
    let reset = move |_| {
        stored.with_value(|cand| {
            if let Some(c) = cand {
                let (l, f) = (lang.get_untracked(), fmt.get_untracked());
                front.set(card_front(c, l, &f));
                back.set(card_back(c, l, &f));
                edited.set(false);
            }
        })
    };

    view! {
        <div class="row" class:is-off=move || !include.get()>
            <label class="row-pick">
                <input type="checkbox" prop:checked=include on:change=move |ev| include.set(event_target_checked(&ev)) />
                <span class="visually-hidden">"Include this card"</span>
            </label>
            <div class="row-body">
                {detail}
                <div class="row-fields">
                    <label class="field field-front">
                        <span class="field-label">"Front"</span>
                        <input
                            id=front_id
                            type="text"
                            lang=move || lang.get().code()
                            prop:value=front
                            on:input=move |ev| {
                                front.set(event_target_value(&ev));
                                edited.set(true);
                            }
                        />
                    </label>
                    <label class="field field-back">
                        <span class="field-label">"Back"</span>
                        <textarea
                            id=back_id
                            rows="2"
                            prop:value=back
                            placeholder=if has_cand { "" } else { "Meaning" }
                            on:input=move |ev| {
                                back.set(event_target_value(&ev));
                                edited.set(true);
                                if !has_cand {
                                    include.set(true);
                                }
                            }
                        ></textarea>
                    </label>
                </div>
                <Show when=move || has_cand && edited.get()>
                    <button type="button" class="link-button" on:click=reset>
                        "Undo my edits"
                    </button>
                </Show>
            </div>
        </div>
    }
}

use crate::model::{Candidate, CardFormat, Lang, ReadingSide, Sense, card_back, card_front};
use leptos::prelude::*;

fn sample(lang: Lang) -> Candidate {
    let s = |tags: &[&str], g: &[&str]| Sense {
        tags: tags.iter().map(|t| t.to_string()).collect(),
        glosses: g.iter().map(|t| t.to_string()).collect(),
    };
    match lang {
        Lang::Zh => Candidate {
            headword: "图书馆".into(),
            reading: "tu2 shu1 guan3".into(),
            variants: vec!["圖書館".into()],
            senses: vec![s(&[], &["library"]), s(&[], &["CL:家[jia1],個|个[ge4]"])],
            common: false,
        },
        Lang::Ja => Candidate {
            headword: "図書館".into(),
            reading: "としょかん".into(),
            variants: vec![],
            senses: vec![s(&["n"], &["library"])],
            common: true,
        },
    }
}

/// A radio group drawn as a row of segments.
#[component]
fn Segments<T>(
    legend: &'static str,
    name: &'static str,
    options: Vec<(T, &'static str)>,
    value: Signal<T>,
    set: Callback<T>,
) -> impl IntoView
where
    T: Copy + PartialEq + Send + Sync + 'static,
{
    view! {
        <fieldset class="segments">
            <legend>{legend}</legend>
            <div class="segments-row">
                {options
                    .into_iter()
                    .map(|(v, label)| {
                        view! {
                            <label class="segment">
                                <input
                                    type="radio"
                                    name=name
                                    prop:checked=move || value.get() == v
                                    on:change=move |_| set.run(v)
                                />
                                <span>{label}</span>
                            </label>
                        }
                    })
                    .collect_view()}
            </div>
        </fieldset>
    }
}

#[component]
pub fn CardOptions(fmt: RwSignal<CardFormat>, lang: RwSignal<Lang>) -> impl IntoView {
    let reading = Signal::derive(move || fmt.get().reading);
    let marks = Signal::derive(move || fmt.get().tone_marks);
    let senses = Signal::derive(move || fmt.get().max_senses);

    let preview = move || {
        let (l, f) = (lang.get(), fmt.get());
        let c = sample(l);
        (card_front(&c, l, &f), card_back(&c, l, &f), l.code())
    };

    view! {
        <aside class="options" aria-labelledby="options-title">
            <h2 id="options-title">"Card layout"</h2>
            <figure class="preview" aria-label="Example card">
                <div class="preview-face preview-front" lang=move || preview().2>{move || preview().0}</div>
                <div class="preview-face preview-back" lang=move || preview().2>{move || preview().1}</div>
            </figure>
            <Segments
                legend="Show the reading on the"
                name="reading"
                options=vec![(ReadingSide::Back, "Back"), (ReadingSide::Front, "Front"), (ReadingSide::Hidden, "Hidden")]
                value=reading
                set=Callback::new(move |v| fmt.update(|f| f.reading = v))
            />
            <Show when=move || lang.get() == Lang::Zh>
                <Segments
                    legend="Pinyin"
                    name="pinyin"
                    options=vec![(true, "Tone marks"), (false, "Numbers")]
                    value=marks
                    set=Callback::new(move |v| fmt.update(|f| f.tone_marks = v))
                />
            </Show>
            <Segments
                legend="Meanings per card"
                name="senses"
                options=vec![(1u8, "1"), (2, "2"), (3, "3"), (0, "All")]
                value=senses
                set=Callback::new(move |v| fmt.update(|f| f.max_senses = v))
            />
            <div class="checks">
                <label class="check">
                    <input
                        type="checkbox"
                        prop:checked=move || fmt.get().show_variants
                        on:change=move |ev| fmt.update(|f| f.show_variants = event_target_checked(&ev))
                    />
                    {move || match lang.get() {
                        Lang::Zh => " Show the other script (simplified or traditional)",
                        Lang::Ja => " Show other spellings",
                    }}
                </label>
                <Show when=move || lang.get() == Lang::Ja>
                    <label class="check">
                        <input
                            type="checkbox"
                            prop:checked=move || fmt.get().show_tags
                            on:change=move |ev| fmt.update(|f| f.show_tags = event_target_checked(&ev))
                        />
                        " Show part of speech"
                    </label>
                </Show>
            </div>
        </aside>
    }
}

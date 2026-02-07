use leptos::prelude::*;

use crate::{
    components::{
        PageTitleComponent,
        flashcard_generation::{ImportGeneratedFlashcards, no_results::NoResultsPage},
    },
    core::flashcard_generation::flashcard::Flashcard,
};

#[component]
pub fn ModifyGeneratedFlashcards(flashcards: Vec<Flashcard>) -> impl IntoView {
    let (data, set_data) = signal(flashcards);
    let (import_flashcards, set_import_flashcards) = signal(false);

    provide_context(data);

    view! {
        <Show
            when=move || { import_flashcards.read() == false }
            fallback=move || view! { <ImportGeneratedFlashcards/> }
        >

            <PageTitleComponent text="Modify Flashcards!"/>

            <Show
                when=move || { !data.read().is_empty() }
                fallback=move || view! { <NoResultsPage/> }
            >
                <div class="text-center m-auto p-2 max-w-7xl">
                    <For
                        each=data
                        key=|state| (state.id.clone(), state.id)
                        let:child
                    >
                        <div class="flex flex-col sm:flex-row gap-2 p-2 border border-primary rounded-lg my-2 shadow-lg">
                            <label class="input flex items-center gap-2 font-bold w-full">
                                "Front"
                                <input type="text" class="grow font-normal" name="front" id="front" value=child.front on:input=move |ev| {
                                    child.front.set(event_target_value(&ev));
                                }/>
                            </label>
                            <label class="input flex items-center gap-2 font-bold grow w-full">
                                "Back"
                                <input type="text" class="grow font-normal" name="back" id="back" value=child.back on:input=move |ev| {
                                    child.back.set(event_target_value(&ev));
                                }/>
                            </label>
                            <button class="btn btn-error" on:click=move |_| {
                                set_data.update(|data| data.retain(|flashcard| flashcard.id != child.id));
                            }>"Delete"</button>
                        </div>
                    </For>
                    <button class="btn btn-primary mt-1 w-full" on:click=move |_| {
                        set_import_flashcards(true)
                    }>"Generate"</button>
                </div>
            </Show>
        </Show>
    }
}

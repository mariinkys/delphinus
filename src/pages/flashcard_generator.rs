use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{
    components::{
        DialogComponent, PageTitleComponent, SelectOption, ToastType,
        flashcard_generation::ModifyGeneratedFlashcards, toast::ToastMessage,
    },
    config::DelphinusConfig,
    core::flashcard_generation::{
        entities::SeparationChar,
        flashcard::{Flashcard, remove_whitespace, search_dictionary},
    },
};

#[component]
pub fn GeneratorPage() -> impl IntoView {
    let set_toast: WriteSignal<ToastMessage> = expect_context();
    let delphinus_config: ReadSignal<DelphinusConfig> = expect_context();

    let (character_string, set_character_string) = signal(String::new());
    let (language, set_language) = signal("Chinese".to_string());
    let (results, set_results) = signal(Vec::<Flashcard>::new());
    let (loading, set_loading) = signal(false);

    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();
        set_loading(true);

        //Check data
        if character_string.read().len() > 0 {
            let is_ch = language.read() == String::from("Chinese");
            spawn_local(async move {
                let clean_input = remove_whitespace(&character_string.get_untracked());
                if !clean_input.is_empty() {
                    match search_dictionary(clean_input, is_ch).await {
                        Ok(found_results) => {
                            set_loading(false);
                            set_results(found_results);
                        }
                        Err(err) => {
                            leptos::logging::error!("{}", err);
                            set_loading(false);
                        }
                    }
                } else {
                    set_loading(false);
                    set_character_string(String::new());
                }
            });
        } else {
            set_loading(false);
        }
    };

    let ocr_dialog_ref_node: NodeRef<leptos::html::Dialog> = NodeRef::new();
    let file_input = NodeRef::<leptos::html::Input>::new();
    let ocr_image = RwSignal::new(None);
    let ocr_upload_loading = RwSignal::new(false);
    let separation_char = RwSignal::new(SeparationChar::default());
    let on_image_submit = move |ev: leptos::ev::SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        if !ocr_upload_loading.get() && ocr_image.get_untracked().is_some() {
            ocr_upload_loading.set(true);
            let separation_char = separation_char.get();

            // TODO: How to make loading appear on submit click (why does it wait for a while? file uploading?)
            spawn_local(async move {
                let image_bytes: Vec<u8> = ocr_image.get_untracked().unwrap();

                match crate::core::flashcard_generation::ocr::ocr_image(
                    image_bytes,
                    separation_char.get_char().to_string(),
                )
                .await
                {
                    Ok(result_string) => {
                        if character_string.get_untracked().is_empty() {
                            set_character_string(result_string);
                        } else {
                            set_character_string
                                .update(|value| *value = format! {"{}{} {}", value, separation_char.get_char(), result_string});
                        };

                        set_toast.set(ToastMessage {
                            message: String::from("Success"),
                            toast_type: ToastType::Success,
                            visible: true,
                        });
                        ocr_image.set(None);
                        ocr_upload_loading.set(false);
                        ocr_dialog_ref_node.get_untracked().unwrap().close();
                    }
                    Err(err) => {
                        set_toast.set(ToastMessage {
                            message: format!("Err {err}"),
                            toast_type: ToastType::Error,
                            visible: true,
                        });
                        ocr_image.set(None);
                        ocr_upload_loading.set(false);
                    }
                }
            });
        }
    };

    view! {
        <Show
            when=move || { results().is_empty() }
            fallback=move || view! { <ModifyGeneratedFlashcards flashcards=results.get()/> }
        >
            <PageTitleComponent text="Generate Flashcards!"/>

            <DialogComponent dialog_title="OCR Image" dialog_node_ref=ocr_dialog_ref_node is_close_btn_disabled={ocr_upload_loading.get()} dialog_content=move || view! {
                <Show when=move || !ocr_upload_loading.get() fallback=move || {
                    view! {
                        <p class="text-center font-bold text-xl">"Processing..."</p>
                        <p class="text-center font-bold text-xl text-error">"Please Wait, do not close this window"</p>
                    }
                }>
                    <form class="flex flex-col gap-3" on:submit=on_image_submit>
                        <fieldset class="fieldset">
                            <label class="label" for="ocr_image">"OCR Image"</label>
                            <input id="ocr_image" disabled=ocr_upload_loading type="file" accept="image/*" class="file-input w-full" node_ref=file_input on:change=move |_ev| {
                                if let Some(files) = file_input.get().unwrap().files()
                                    && let Some(file) = files.get(0) {
                                        let file_type = crate::core::utils::is_extension_image(&file);
                                        if file_type.is_none() {
                                            set_toast.set(ToastMessage {
                                                message: String::from("Not a valid image"),
                                                toast_type: ToastType::Error,
                                                visible: true,
                                            });
                                            ocr_image.set(None);
                                        } else {
                                            spawn_local(async move {
                                                let promise = file.array_buffer();
                                                if let Ok(js_value) = wasm_bindgen_futures::JsFuture::from(promise).await {
                                                    let bytes = web_sys::js_sys::Uint8Array::new(&js_value).to_vec();
                                                    ocr_image.set(Some(bytes));
                                                } else {
                                                    ocr_image.set(None);
                                                }
                                            });
                                        }
                                    }
                            }
                        />
                        </fieldset>

                        <fieldset class="fieldset">
                            <label class="label" for="theme">"Separation Char"</label>
                            <select
                                class="select select-primary w-full"
                                id="spearation_char"
                                on:change=move |ev| {
                                    if let Ok(new_char) = event_target_value(&ev).parse::<SeparationChar>() {
                                        separation_char.set(new_char);
                                    }
                                }
                            >
                                <For
                                    each=move || SeparationChar::ALL.iter()
                                    key=|s_char| *s_char
                                    children=move |s_char| {
                                        let is_selected = move || separation_char.get() == *s_char;

                                        view! {
                                            <option
                                                value=s_char.to_string()
                                                selected=is_selected
                                            >
                                                {s_char.to_string()}
                                            </option>
                                        }
                                    }
                                />
                            </select>
                        </fieldset>

                        <button disabled=ocr_upload_loading class="btn btn-primary mt-3 w-full" type="submit">"Upload"</button>
                    </form>
                </Show>
            }/>

            <div class="max-w-4xl text-center m-auto p-2">
                <div class="flex justify-between my-3 items-center">
                    <button
                        disabled=move || {delphinus_config.get().disable_ocr}
                        class="btn btn-sm btn-accent"
                        on:click=move |_| {
                            let _ = ocr_dialog_ref_node.get().unwrap().show_modal();
                        }
                    >"OCR"</button>
                    <div class="tooltip tooltip-left" data-tip="For Japanese characters, you can separate the characters with the Japanese '、' for Chinese characters, you can use the Chinese '，' for both of them, you can use the traditional ',' however, you cannot mix them within the same input.">
                        <button class="btn btn-xs">"i"</button>
                    </div>
                </div>

                <form class="w-full" on:submit=on_submit>
                    <textarea
                        class="textarea textarea-primary w-full h-80" placeholder="Enter character string"
                        prop:value=move || character_string.get()
                        on:input=move |ev| { set_character_string(event_target_value(&ev)) }
                        disabled=loading
                    >
                    </textarea>
                    <br/>
                    <select
                        class="select select-primary w-full mt-3"
                        on:change=move |ev| {
                            let new_language = event_target_value(&ev);
                            set_language(new_language);
                        }
                        disabled=loading
                    >
                        <SelectOption value=language is="Chinese"/>
                        <SelectOption value=language is="Japanese"/>
                    </select>
                    <br/>
                    <button class="btn btn-primary mt-3 w-full" type="submit" disabled=loading>"Generate"</button>
                </form>
            </div>
        </Show>

    }
}

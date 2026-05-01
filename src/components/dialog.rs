use leptos::prelude::*;

#[component]
pub fn DialogComponent(
    /// In case you don't provide a dialog_node_ref this is the text that will be inside the default component button, it will default to 'Open Dialog' if not provided.
    #[prop(optional, into)]
    btn_text: String,
    /// In case you don't provide a dialog_node_ref this are the classes that will apply to the default component button, it will default to 'btn btn-secondary'.
    #[prop(optional, into)]
    btn_class: String,
    /// The title of the dialog, it will default to 'Dialog Title' if not provided.
    #[prop(optional, into)]
    dialog_title: String,
    /// If you want to control the state of the close button enabled(false)/disabled(true), if not provided btn is always active
    #[prop(optional, into)]
    is_close_btn_disabled: Option<bool>,
    /// The NodeRef that let's you control the dialog, if not provided it will be managed by the component itself, the component will default to a button that opens the dialog.
    /// You can provide the NodeRef and control it from outside the component; providing the NodeRef will make it so that the component does not render the default button that opens the dialog.
    /// If you provide you're own NodeRef you can open the dialog using: `dialog_ref.get().unwrap().show_modal();`
    #[prop(optional)]
    dialog_node_ref: Option<NodeRef<leptos::html::Dialog>>,
    /// A closure that returns what gets rendered inside the dialog.
    #[prop(into)]
    dialog_content: ViewFn,
) -> impl IntoView {
    let dialog_ref = if let Some(dialog) = dialog_node_ref {
        dialog
    } else {
        NodeRef::new()
    };

    let button_text = if btn_text.is_empty() {
        String::from("Open Dialog")
    } else {
        btn_text
    };

    let button_class = if btn_class.is_empty() {
        String::from("btn btn-secondary")
    } else {
        btn_class
    };

    let dia_title = if dialog_title.is_empty() {
        String::from("Dialog Title")
    } else {
        dialog_title
    };

    let close_btn_disabled = is_close_btn_disabled.unwrap_or_default();

    view! {
        <Show when=move || dialog_node_ref.is_none()>
            <button
                class={button_class.to_string()}
                on:click=move |_| {
                    let _ = dialog_ref.get().unwrap().show_modal();
                }
            >
                {button_text.to_string()}
            </button>
        </Show>

        <dialog class="modal" node_ref=dialog_ref>
          <div class="modal-box">
            <div class="flex justify-between items-center">
                <h2 class="text-2xl font-bold">{dia_title.to_string()}</h2>
                <button
                    disabled=close_btn_disabled
                    class="btn btn-sm btn-ghost"
                    on:click=move |_| {
                        dialog_ref.get().unwrap().close();
                    }
                >
                    "X"
                </button>
            </div>
            <br/>

            {dialog_content.run()}

          </div>
        </dialog>
    }
}

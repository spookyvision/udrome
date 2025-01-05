use dioxus::prelude::*;
#[component]
pub fn Player(
    url: String,
    title: String,
    onmounted: EventHandler<MountedEvent>,
    onfocus: EventHandler<FocusEvent>,
    onblur: EventHandler<FocusEvent>,
) -> Element {
    rsx! {
        div { id: "player", class: "fixed bottom-0 z-41 h-10",
            audio {
                class: "w-128",
                onmounted,
                onfocus,
                onblur,

                controls: true,
                source { src: url, r#type: "audio/mpeg" }
            }
            "{title}"
        }
    }
}

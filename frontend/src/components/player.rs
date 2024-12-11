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
            div {
                id: "player",
                audio {
                    onmounted,
                    onfocus,
                    onblur,

                    controls:true, source{
                    src:url, type:"audio/mpeg"
                },
            }
            "{title}"
        }
    }
}

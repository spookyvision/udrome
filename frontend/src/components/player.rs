use dioxus::{prelude::*, web::WebEventExt};
use dioxus_logger::tracing::{debug, warn};
use wasm_bindgen::JsCast;
use web_sys::HtmlAudioElement;
#[component]
pub fn Player(url: String, title: String) -> Element {
    rsx! {
            div {
                id: "player",
                audio {
                    onmounted: move|el: MountedEvent| {
                        if let Some(el) = el.try_as_web_event() {
                            if let Ok(el) = el.dyn_into::<HtmlAudioElement>() {
                                el.load();
                                if let Err(e) = el.play() {
                                    warn!("player error: {e:?}");
                                }
                            }
                        }
                    },

                    controls:true, source{
                    src:url, type:"audio/mpeg"
                },
            }
            "{title}"
        }
    }
}

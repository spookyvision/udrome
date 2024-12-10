use dioxus::{prelude::*, web::WebEventExt};
use dioxus_logger::tracing::warn;
use wasm_bindgen::JsCast;
use web_sys::HtmlAudioElement;
#[component]
pub fn Player(url: String, title: String) -> Element {
    let mut this = use_signal(|| None);
    rsx! {
            div {
                id: "player",
                audio {
                    onmounted: move|ev: MountedEvent| {
                        if let Some(el) = ev.try_as_web_event() {
                            if let Ok(el) = el.dyn_into::<HtmlAudioElement>() {
                                el.load();
                                if let Err(e) = el.play() {
                                    warn!("player error: {e:?}");
                                }
                                this.set(Some(el));
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

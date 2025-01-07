use dioxus::{prelude::*, web::WebEventExt};
use dioxus_logger::tracing::warn;
use wasm_bindgen::JsCast;
use web_sys::HtmlAudioElement;

use crate::model::globals::{BaseUrl, PLAYER, SONG};
#[component]
pub fn Player(onfocus: EventHandler<FocusEvent>, onblur: EventHandler<FocusEvent>) -> Element {
    let song_lock = SONG.read();
    let song = song_lock.as_ref();
    let title = song
        .map(|s| s.title_with_optional_artist())
        .unwrap_or_default();
    let base_url = use_context::<BaseUrl>();
    let src = song.map(|s| s.stream_url(&base_url)).unwrap_or_default();

    let onmounted = {
        to_owned![src];
        move |ev: MountedEvent| {
            if let Some(el) = ev.try_as_web_event() {
                if let Ok(el) = el.dyn_into::<HtmlAudioElement>() {
                    if !src.is_empty() {
                        el.load();
                        if let Err(e) = el.play() {
                            warn!("player error: {e:?}");
                        }
                    }
                    *PLAYER.write() = Some(el);
                }
            }
        }
    };
    rsx! {
        div { id: "player", class: "sm:ml-64 fixed bottom-0 z-41 h-10",
            audio {
                class: "w-128 inline-block border-r-1 border-slate-700 mr-4",
                onmounted,
                onfocus,
                onblur,

                controls: true,
                source { src, r#type: "audio/mpeg" }
            }
            "{title}"
        }
    }
}

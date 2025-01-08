use dioxus::{prelude::*, web::WebEventExt};
use dioxus_logger::tracing::debug;

use crate::model::{globals::SONG, SongInfo};
#[component]
pub fn SearchResult(
    content: ReadOnlySignal<Vec<SongInfo>>,
    onclick: EventHandler<SongInfo>,
    offset: usize,
) -> Element {
    let cur_song = SONG.read();
    let content_lock = content.read();
    let song_rows = content_lock.iter().enumerate().map(|(idx, song)| {
        to_owned![song];
        let row_is_current_song = cur_song
            .as_ref()
            .map(|s| s.id == song.id)
            .unwrap_or_default();
        let class = if row_is_current_song {
            "cursor-pointer hover:bg-base-200 font-bold text-slate-100"
        } else {
            "cursor-pointer hover:bg-base-200"
        };
        rsx! {
            tr {
                key: "{song.id}",
                class,
                draggable: true,
                ondragstart: move |ev| {
                    debug!("ondragstart {ev:?}");
                    if let Some(wev) = ev.try_as_web_event() {
                        debug!("haev {wev:?}");
                        if let Some(dt) = wev.data_transfer() {
                            debug!("haev {dt:?}");
                            dt.set_drop_effect("move");
                            dt.set_effect_allowed("move");
                        }
                    }
                },
                ondrop: move |ev| {
                    ev.prevent_default();
                },
                ondragover: move |ev| {
                    ev.prevent_default();
                },
                onclick: move |_| onclick.call(song.clone()),
                td { "{ idx + offset + 1 }" }
                td { "{ song.artist.clone().unwrap_or_default() }" }
                td { "{ song.title }" }
                td { "{ song.album.clone().unwrap_or_default() }" }
                td { "{ song.duration_humanized() }" }
                td {
                    div { class: "rating rating-sm",
                        input {
                            name: "rating-{idx}",
                            r#type: "radio",
                            class: "mask mask-star",
                        }
                        input {
                            name: "rating-{idx}",
                            r#type: "radio",
                            class: "mask mask-star",
                        }
                        input {
                            name: "rating-{idx}",
                            r#type: "radio",
                            class: "mask mask-star",
                        }
                        input {
                            name: "rating-{idx}",
                            r#type: "radio",
                            checked: true,
                            class: "mask mask-star",
                        }
                        input {
                            r#type: "radio",
                            name: "rating-{idx}",
                            class: "mask mask-star",
                        }
                    }
                
                }
            }
        }
    });
    // thead z-1 is required so rating stars stay clipped below
    rsx! {
        div { class: "sm:mb-10 sm:mt-10 overflow-y-auto h-[calc(100vh-5rem)] border-y-1 border-slate-700",
            table { class: "table",
                thead { class: "sticky top-0 bg-base-300 z-1",
                    tr {
                        th { "#" }
                        th { "Artist" }
                        th { "Song" }
                        th { "Album" }
                        th { "Length" }
                        th { "Rating" }
                    }
                }
                tbody { class: "text-slate-300", {song_rows} }
            }
        }
    }
}

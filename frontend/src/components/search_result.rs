use dioxus::prelude::*;
use subsonic_types::response::{Child as Song, Response, ResponseBody};

use crate::model::SongInfo;

#[component]
pub fn SearchResult(
    content: ReadOnlySignal<Vec<SongInfo>>,
    onclick: EventHandler<SongInfo>,
    offset: usize,
) -> Element {
    let content_lock = content.read();
    let song_rows = content_lock.iter().enumerate().map(|(idx, song)| {
        to_owned![song];
        rsx! {
            tr {
                key: "{song.id}",
                class: "cursor-pointer hover:bg-base-200",
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
        div { class: "pl-8 sm:mb-10 sm:mt-10 overflow-y-auto h-[calc(100vh-5rem)]",
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

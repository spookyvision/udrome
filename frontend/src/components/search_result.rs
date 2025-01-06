use dioxus::prelude::*;
use subsonic_types::response::{Child as Song, Response, ResponseBody};

use crate::model::SongInfo;

#[component]
pub fn SearchResult(content: Signal<Option<Response>>, onclick: EventHandler<SongInfo>) -> Element {
    let html = r#"
    div { class: "rating",
        input { name: "rating-{idx}", r#type: "radio", class: "mask mask-star" }
        input {
            name: "rating-{idx}",
            checked: "checked",
            r#type: "radio",
            class: "mask mask-star",
        }
        input { name: "rating-{idx}", r#type: "radio", class: "mask mask-star" }
        input { name: "rating-{idx}", r#type: "radio", class: "mask mask-star" }
        input { r#type: "radio", name: "rating-{idx}", class: "mask mask-star" }
    }"#;
    rsx! {
        div { class: "pl-8 sm:mb-10 sm:mt-10 overflow-y-auto h-[calc(100vh-5rem)]",
            match content.read().as_ref().map(|res| &res.body) {
                Some(ResponseBody::SearchResult3(res)) => {
                    rsx! {
                        table { class: "table",
                            thead {
                                tr {
                                    th { "#" }
                                    th { "Artist" }
                                    th { "Song" }
                                    th { "Album" }
                                    th { "Length" }
                                    th { "Rating" }
                                }
                            }
                            tbody {
                                for (idx , song , song2) in res.song
                                    .iter()
                                    .cloned()
                                    .enumerate()
                                    .map(|(idx, song)| { (idx, SongInfo(song.clone()), song) })
                                {
                                    tr {
                                        key: "{song.id}",
                                        class: "cursor-pointer hover:bg-base-200",
                                        onclick: move |_| onclick.call(SongInfo(song2.clone())),
                                        td { "{ idx + 1 }" }
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
                            }
                        }
                    }
                }
                Some(_) => rsx! { "borked response" },
                None => rsx! { "no response?" },
            }
        }
    }
}

use dioxus::prelude::*;
use subsonic_types::response::{Child as Song, Response, ResponseBody};

#[component]
pub fn SearchResult(content: Signal<Option<Response>>, onclick: EventHandler<Song>) -> Element {
    rsx! {
        div { class: "pl-8 sm:mb-10 sm:mt-10 overflow-y-auto h-[calc(100vh-6rem)]",
            match content.read().as_ref().map(|res| &res.body) {
                Some(ResponseBody::SearchResult3(res)) => {
                    rsx! {
                        ul {
                        
                            for (song , display) in res.song
                                .iter()
                                .cloned()
                                .map(|song| {
                                    let display = if let Some(artist) = &song.artist {
                                        format!("{artist} - {}", song.title)
                                    } else {
                                        song.title.clone()
                                    };
                                    (song, display)
                                })
                            {
                                li {
                                    key: "{song.id}",
                                    class: "cursor-pointer",
                                    onclick: move |_| onclick.call(song.clone()),
                                    "{display}"
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

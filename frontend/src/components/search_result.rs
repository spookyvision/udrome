use dioxus::prelude::*;
use subsonic_types::response::{Response, ResponseBody};

#[component]
pub fn SearchResult(content: Signal<Option<Response>>) -> Element {
    rsx! {
        match content.read().as_ref().map(|res| &res.body) {
            Some(ResponseBody::SearchResult3(res)) => rsx! {
                ul {
                    for song in &res.song {
                        li { "{song.title}" }
                    }
                }
            },
            Some(_) => rsx! { "borked response" },
            None => rsx! { "no response?" },
        }
    }
}

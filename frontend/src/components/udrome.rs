use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_logger::tracing::{debug, error, info};
use futures::StreamExt;
use serde::Deserialize;
use subsonic_types::{
    common::{Format, Version},
    request::{search::Search3, Authentication, Request as SRequest, SubsonicRequest},
    response::{Response as SubsonicResponse, ResponseBody, SearchResult3},
};
use url::Url;

use crate::components::SearchResult;

const HEADER_SVG: Asset = asset!("/assets/header.svg");

struct Paginator {
    pos: u32,
    size: u32,
}

impl Default for Paginator {
    fn default() -> Self {
        Self::new(50)
    }
}
impl Paginator {
    fn new(size: u32) -> Self {
        Self { pos: 0, size }
    }

    fn cur(&self) -> u32 {
        self.pos
    }
    fn prev(&mut self) -> u32 {
        self.pos = self.pos.saturating_sub(self.size);
        self.pos
    }
    fn next(&mut self) -> u32 {
        self.pos = self.pos.saturating_add(self.size);
        self.pos
    }
}

#[derive(Debug, Deserialize)]
struct SubsonicResponseOuter {
    #[serde(rename = "subsonic-response")]
    subsonic_response: SubsonicResponse,
}

struct Request00r {
    paginator: Paginator,
    base_url: String,
}

impl Request00r {
    fn new(paginator: Paginator, base_url: impl AsRef<str>) -> Self {
        Self {
            paginator,
            base_url: base_url.as_ref().to_string(),
        }
    }

    fn cur(&self) -> Result<Url, url::ParseError> {
        let search = Search3 {
            query: "".to_string(),
            song_count: Some(self.paginator.size),
            song_offset: Some(self.paginator.pos),
            music_folder_id: None,
            artist_count: None,
            artist_offset: None,
            album_count: None,
            album_offset: None,
        };

        let request = SRequest {
            username: "todo".into(),
            authentication: Authentication::Password("todo".into()),
            version: Version::LATEST,
            client: "udrome-dx".into(),
            format: Some(Format::Json.to_string()),
            body: search,
        };
        // TODO ergonomics?
        let path = SRequest::<Search3>::PATH;
        let query = request.to_query();

        // TODO .view?
        let res = format!("{}{path}.view?{query}", self.base_url).parse();
        res
    }
}

#[component]
pub fn Udrome() -> Element {
    let mut response_state = use_signal(|| None);
    let base_url = "http://localhost:3000";
    let req = use_signal(|| Request00r::new(Paginator::default(), base_url));
    // TODO error handling and/or (better?) assume always valid via validating base_url first
    let tx = use_coroutine(move |mut rx| async move {
        // Define your state before the loop
        let client = reqwest::Client::new();
        let mut cache: HashMap<Url, SubsonicResponse> = HashMap::new();

        loop {
            // Loop and wait for the next message
            if let Some(url) = rx.next().await {
                info!("felch {url}");
                // Resolve the message
                let response = if let Some(response) = cache.get(&url) {
                    response.clone()
                } else {
                    let response: SubsonicResponseOuter = client
                        .get(url.clone())
                        .send()
                        .await
                        .inspect_err(|e| error!("noes! {e:?}"))
                        .unwrap()
                        .json()
                        .await
                        .unwrap();
                    let response = response.subsonic_response;
                    cache.insert(url, response.clone());
                    response
                };
                response_state.set(Some(response));
            } else {
                break;
            }
        }
    });

    let first_req = Request00r::new(Paginator::default(), base_url)
        .cur()
        .unwrap();

    // Send a message to the coroutine
    tx.send(first_req.clone());
    // Get the current state of the coroutine
    // let response = response_state.read();

    // let dom = match response.as_ref().map(|res| &res.body) {
    //     Some(ResponseBody::SearchResult3(res)) => rsx! {
    //         ul {
    //             for song in &res.song {
    //                 li { "{song.title}" }
    //             }
    //         }
    //     },
    //     Some(_) => rsx! { "borked response" },
    //     None => rsx! { "no response?" },
    // };

    rsx! {
        div { id: "hero",
            SearchResult { content: response_state }
        }
    }
}

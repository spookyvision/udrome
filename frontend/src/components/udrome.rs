use std::{collections::HashMap, time::Duration};

use dioxus::prelude::*;
use dioxus_cli_config::is_cli_enabled;
use dioxus_logger::tracing::{debug, error, info};
use futures::StreamExt;
use serde::Deserialize;
use subsonic_types::{
    common::{Format, Version},
    request::{search::Search3, Authentication, Request as SRequest, SubsonicRequest},
    response::{Child as Song, Response as SubsonicResponse},
};
use url::Url;

use crate::{
    components::{Player, SearchResult},
    sdk::debounce::use_debounce,
};

struct Paginator {
    offset: u32,
    size: u32,
    search: Option<String>,
}

impl Default for Paginator {
    fn default() -> Self {
        Self::new(50)
    }
}
impl Paginator {
    fn new(size: u32) -> Self {
        Self {
            offset: 0,
            size,
            search: None,
        }
    }

    fn cur(&self) -> Page {
        Page {
            size: self.size,
            offset: self.offset,
            search: self.search.clone(),
        }
    }
    fn at(&mut self, at: u32) -> Page {
        self.offset = at * self.size;
        self.cur()
    }
    fn prev(&mut self) -> Page {
        self.offset = self.offset.saturating_sub(self.size);
        self.cur()
    }
    fn next(&mut self) -> Page {
        self.offset = self.offset.saturating_add(self.size);
        self.cur()
    }

    fn set_search(&mut self, search: Option<String>) {
        self.search = search;
    }
}

#[derive(Debug, Deserialize)]
struct SubsonicResponseOuter {
    #[serde(rename = "subsonic-response")]
    subsonic_response: SubsonicResponse,
}

struct Request00r {
    base_url: String,
}

struct Page {
    size: u32,
    offset: u32,
    search: Option<String>,
}

impl Request00r {
    fn new(base_url: impl AsRef<str>) -> Self {
        Self {
            base_url: base_url.as_ref().to_string(),
        }
    }

    fn at(&self, page: u32, size: u32, query: Option<String>) -> Result<Url, url::ParseError> {
        let search = Search3 {
            query: query.unwrap_or_default(),
            song_count: Some(size),
            song_offset: Some(page),
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

enum Command {
    Next,
    Prev,
    At(u32),
}
#[component]
pub fn Udrome() -> Element {
    let standalone = if is_cli_enabled() { "no" } else { "yes" };
    let mut response_state = use_signal(|| None);
    let mut paginator = use_signal(|| Paginator::default());
    let mut song_url = use_signal(|| "".to_string());
    let mut title = use_signal(|| "".to_string());
    let mut search = use_signal(|| None);
    let base_url = use_signal(|| {
        option_env!("BACKEND_URL")
            .map(|e| e.to_string())
            .unwrap_or_else(|| {
                web_sys::window()
                    .map(|win| win.location().origin().ok())
                    .flatten()
                    .expect("could not determine origin URL")
            })
    });

    let mut debounce = use_debounce(Duration::from_millis(100), move |text: String| {
        debug!("{text}");
        let search_val = if !text.is_empty() {
            Some(text.clone())
        } else {
            None
        };
        search.set(search_val);
    });
    let tx = use_coroutine(move |mut rx: UnboundedReceiver<Page>| async move {
        // TODO hardcoded lol
        let req = Request00r::new(base_url.read().as_str());

        // Define your state before the loop
        let client = reqwest::Client::new();
        let mut cache: HashMap<Url, SubsonicResponse> = HashMap::new();

        loop {
            // Loop and wait for the next message
            if let Some(page) = rx.next().await {
                // TODO error handling and/or (better?) assume always valid via validating base_url first
                let Ok(url) = req.at(page.offset, page.size, page.search) else {
                    continue;
                };

                // Resolve the message
                let response = if let Some(response) = cache.get(&url) {
                    response.clone()
                } else {
                    let response: SubsonicResponseOuter = client
                        .get(url.clone())
                        .send()
                        .await
                        .inspect_err(|e| error!("oh nose {e:?}"))
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

    // Send a message to the coroutine
    let _ = use_resource(move || async move {
        debug!("update");
        paginator.write().set_search(search.read().clone());
        tx.send(paginator.read().cur());
    });

    rsx! {
        div { id: "udrome",
            div { class: "mx-auto px-4 fixed overflow-y-auto",
                input {
                    oninput: move |ev| {
                        let text = ev.value();
                        debounce.action(text);
                    }
                }
                button {
                    class: "btn",
                    onclick: move |ev| {
                        paginator.write().prev();
                    },
                    "prev"
                }
                button {
                    class: "btn",
                    onclick: move |ev| {
                        paginator.write().next();
                    },
                    "next"
                }
                Player { url: song_url, title }
                "standalone build: {standalone}"
            }

            SearchResult {
                content: response_state,
                onclick: move |song: Song| {
                    to_owned!(base_url);
                    let url = format!("{}/rest/stream.view?id={}", base_url, song.id);
                    title.set(song.title);
                    debug!("play {url}");
                    song_url.set(url);
                }
            }
        }
    }
}

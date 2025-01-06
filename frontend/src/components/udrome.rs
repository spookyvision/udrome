use std::{collections::HashMap, time::Duration};

use dioxus::{prelude::*, web::WebEventExt};
use dioxus_logger::tracing::{debug, error, warn};
use futures::StreamExt;
use serde::Deserialize;
use subsonic_types::{
    common::{Format, Version},
    request::{search::Search3, Authentication, Request as SRequest, SubsonicRequest},
    response::{Child as Song, Response as SubsonicResponse, ResponseBody},
};
use url::Url;
use wasm_bindgen::JsCast;
use web_sys::{HtmlAudioElement, HtmlElement, HtmlInputElement};

use crate::{
    components::{Player, SearchResult},
    model::SongInfo,
    sdk::debounce::use_debounce,
};

#[derive(Debug, PartialEq, Eq)]
struct Paginator {
    offset: u32,
    max_offset: u32,
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
            max_offset: u32::MAX,
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
        self.offset = self.offset.saturating_add(self.size).min(self.max_offset);
        self.cur()
    }

    fn set_search(&mut self, search: Option<String>) {
        if search != self.search {
            self.offset = 0;
            self.max_offset = u32::MAX;
            self.search = search;
        }
    }

    // limit max offset if we're on the last page or overshot it
    fn clamp_offset(&mut self, cur_result_count: u32) {
        if cur_result_count < self.size {
            debug!(
                "cur {} has {cur_result_count} results, clamping to cur",
                self.offset
            );
            self.max_offset = self.offset;
        } else if cur_result_count == 0 {
            // TODO this branch needs testing
            debug!("cur {} has 0 results, clamping -1", self.offset);
            self.max_offset = self.offset.saturating_sub(1);
        }
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

#[derive(Debug, PartialEq, Eq)]
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

    // TODO maybe? https://crates.io/crates/dioxus-query
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

#[component]
pub fn Udrome() -> Element {
    debug!("rerender");
    let mut response_state = use_signal(|| None);
    let mut paginator = use_signal(|| Paginator::default());
    let mut song_url = use_signal(|| "".to_string());
    let mut title = use_signal(|| "".to_string());
    let mut search = use_signal(|| None);
    let mut search_field: Signal<Option<HtmlInputElement>> = use_signal(|| None);
    let mut app_container: Signal<Option<HtmlElement>> = use_signal(|| None);
    let mut player = use_signal(|| None);
    let mut player_focused = use_signal(|| false);
    let mut search_focused = use_signal(|| false);

    let base_url: Signal<String> = use_signal(|| {
        option_env!("BACKEND_URL")
            .map(|e| e.to_string())
            .unwrap_or_else(|| {
                web_sys::window()
                    .map(|win| win.location().href().inspect_err(|e| error!("{e:?}")).ok())
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
    // TODO maybe use_resource?
    let tx = use_coroutine(move |mut rx: UnboundedReceiver<Page>| async move {
        let req = Request00r::new(base_url.read().as_str());

        // Define your state before the loop
        let client = reqwest::Client::new();
        let mut cache: HashMap<Url, SubsonicResponse> = HashMap::new();

        loop {
            // Loop and wait for the next message
            if let Some(page) = rx.next().await {
                debug!("request {page:?}");
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
                        .unwrap() // TODO this crashes on error
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

    let _ = use_resource(move || async move {
        debug!("update");
        paginator.write().set_search(search.read().clone());
        tx.send(paginator.read().cur());
    });

    let response_memo = use_memo(move || response_state());
    let _ = use_resource(move || async move {
        if let Some(response) = response_memo.read().as_ref() {
            if let ResponseBody::SearchResult3(res) = &response.body {
                let count = res.song.iter().count();
                // SAFETY: force-converting usize to u32:
                // search results are limited to ~a single visible page. This should
                // comfortably fit into the destination type unless you're an alien with
                // frightening vision capabilities
                paginator.write().clamp_offset(count.try_into().unwrap());
            }
        }
    });

    let player_mounted = move |ev: MountedEvent| {
        if let Some(el) = ev.try_as_web_event() {
            if let Ok(el) = el.dyn_into::<HtmlAudioElement>() {
                el.load();
                if let Err(e) = el.play() {
                    warn!("player error: {e:?}");
                }
                player.set(Some(el));
            }
        }
    };

    // handler defined as closure so we can have comments (rsx format removes them)
    let onkeydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        let code = ev.code();
        let mofos = ev.modifiers();
        debug!(">{key}< >{code}< {mofos:?}");
        let mut handled = false;
        // TODO howto i18n search?
        // TODO properly handle Mac (meta = cmd) vs non-Mac
        // there is Key::Find but it doesn't seem to trigger
        if key == Key::Character("f".to_string()) && (mofos.ctrl() || mofos.meta()) {
            if let Some(search_field) = search_field.as_ref() {
                if let Err(e) = search_field.focus() {
                    error!("{e:?}");
                }
                handled = true;
            }
        } else if key == Key::Escape {
            // clear search input
            if let Some(search_field) = search_field.as_ref() {
                search_field.set_value("");
            }
            // force search action update
            debounce.action("".to_string());
            // change focus to app
            // TODO reset scroll position
            if let Some(app) = app_container.as_ref() {
                app.focus().inspect_err(|e| error!("{e:?}")).ok();
            }
            handled = true;
        } else if code == Code::Space {
            // we want space to work normally in the search field,
            // and the player element normally has its own playpause on space
            if !*search_focused.read() && !*player_focused.read() {
                if let Some(player) = player.as_ref() {
                    if player.paused() {
                        player.play().inspect_err(|e| error!("{e:?}")).ok();
                    } else {
                        player.pause().inspect_err(|e| error!("{e:?}")).ok();
                    }
                }
                handled = true;
            }
        }

        if handled {
            ev.prevent_default();
        }
    };

    // TODO https://crates.io/crates/dioxus-free-icons
    // TODO why do we need overflow-y-hidden ??

    rsx! {
        div {
            tabindex: 0,
            onkeydown,
            id: "udrome",
            class: "sm:ml-64 h-screen overflow-y-hidden",
            onmounted: move |ev| {
                if let Some(el) = ev.try_as_web_event() {
                    if let Ok(el) = el.dyn_into::<HtmlElement>() {
                        app_container.set(Some(el))
                    }
                }
            },
            div { class: "px-4 top-0 fixed h-10",
                input {
                    onmounted: move |ev| {
                        debug!("mounted {ev:?}");
                        if let Some(el) = ev.try_as_web_event() {
                            if let Ok(el) = el.dyn_into::<HtmlInputElement>() {
                                search_field.set(Some(el))
                            }
                        }
                    },
                    onfocus: move |_ev| {
                        debug!("search::focus");
                        search_focused.set(true);
                    },
                    onblur: move |_ev| {
                        debug!("search::blur");
                        search_focused.set(false);
                    },
                    oninput: move |ev| {
                        let text = ev.value();
                        debounce.action(text);
                    },
                }
                button {
                    class: "btn",
                    onclick: move |_ev| {
                        paginator.write().prev();
                    },
                    "prev"
                }
                button {
                    class: "btn",
                    onclick: move |_ev| {
                        paginator.write().next();
                    },
                    "next"
                }
            }

            SearchResult {
                content: response_state,
                onclick: move |song: SongInfo| {
                    let base_url = base_url.read();
                    let bu = base_url.as_str();
                    let stream_url = song.stream_url(bu);
                    debug!("play {stream_url}");
                    if let Some(cover) = song.cover_art_url(bu) {
                        debug!("ca {cover}");
                    }
                    title.set(song.title.clone());
                    song_url.set(stream_url);
                },
            }

            Player {
                url: song_url,
                title,
                onmounted: player_mounted,
                onfocus: move |_ev| {
                    debug!("player::focus");
                    player_focused.set(true);
                },
                onblur: move |_ev| {
                    debug!("player::blur");
                    player_focused.set(false);
                },
            }
        }
    }
}

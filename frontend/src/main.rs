use components::{Navbar, Player};
use dioxus::prelude::*;
use dioxus_logger::tracing::{debug, error, warn, Level};
use model::globals::{BaseUrl, Command, Focus, FOCUS, PLAYER};
use views::{Album, Albums, Artist, Artists, Home, Playlist, Playlists, Song};
use wasm_bindgen::{prelude::Closure, JsCast};

mod components;
mod model;
mod sdk;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home { },
    #[route("/songs/:id")]
    Song { id: i32 },
    #[route("/artists/")]
    Artists {},
    #[route("/artists/:id")]
    Artist { id: i32 },
    #[route("/albums/")]
    Albums {},
    #[route("/albums/:id")]
    Album { id: i32 },
    #[route("/playlists/")]
    Playlists {},
    #[route("/playlists/:id")]
    Playlist { id: i32 },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus_logger::init(Level::DEBUG).expect("failed to init logger");
    dioxus::launch(App);
}

// TODO https://crates.io/crates/dioxus-i18n

#[component]
fn App() -> Element {
    warn!("app rerun");

    let (tx, _rx) = use_context_provider(|| async_channel::bounded::<Command>(2));
    use_context_provider(|| {
        let mut base_url = option_env!("BACKEND_URL")
            .map(|e| e.to_string())
            .unwrap_or_else(|| {
                web_sys::window()
                    .map(|win| {
                        win.location()
                            .href()
                            .inspect_err(|e| error!("base_url: {e:?}"))
                            .ok()
                    })
                    .flatten()
                    .expect("could not determine origin URL")
            });

        base_url = base_url
            .strip_suffix("/")
            .map(|s| s.to_string())
            .unwrap_or(base_url);

        BaseUrl(base_url)
    });

    use_effect(move || {
        let body = gloo::utils::body();
        let cb =
            Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                debug!("body okd {ev:?}");
            });
        body.set_onkeydown(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: TAILWIND_CSS }

        // TODO onkeydown only fires after clicking the same menu item twice?
        div {
            class: "app-wrapper h-screen",
            tabindex: 1,
            onkeydown: {
                move |ev: KeyboardEvent| {
                    to_owned![tx];
                    async move {
                        let key = ev.key();
                        let code = ev.code();
                        let mofos = ev.modifiers();
                        debug!(">{key}< >{code}< {mofos:?}");
                        let mut handled = false;
                        if key == Key::Character("f".to_string())
                            && (mofos.ctrl() || mofos.meta())
                        {
                            tx.send(Command::FocusSearch)
                                .await
                                .inspect_err(|e| error!("command-chan is sick {e:?}"))
                                .ok();
                            handled = true;
                        } else if key == Key::Escape {
                            tx.send(Command::BlurSearch)
                                .await
                                .inspect_err(|e| error!("command-chan is sick {e:?}"))
                                .ok();
                            handled = true;
                        } else if code == Code::Space {
                            let focus = *FOCUS.read();
                            match focus {
                                Some(focus) => {
                                    debug!("on space: focus is {focus:?}, doing nothing");
                                }
                                None => {
                                    let player = PLAYER.read();
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
                        }
                        if handled {
                            ev.prevent_default();
                            ev.stop_propagation();
                        }
                    }
                }
            },
            Router::<Route> {}
            Player {
                onfocus: move |_ev| {
                    *FOCUS.write() = Some(Focus::Player);
                },
                onblur: move |_ev| {
                    *FOCUS.write() = None;
                },
            }
        }
    }
}

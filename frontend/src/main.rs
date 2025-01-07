use components::Navbar;
use dioxus::prelude::*;
use dioxus_logger::tracing::Level;
use views::{Album, Albums, Artist, Artists, Home, Playlist, Playlists, Song};

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
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

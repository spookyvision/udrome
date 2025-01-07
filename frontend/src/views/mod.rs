mod home;
use dioxus::prelude::*;
pub use home::Home;

#[component]
pub fn Song(id: i32) -> Element {
    rsx! { "song {id}" }
}

#[component]
pub fn Artists() -> Element {
    rsx! { "artists" }
}

#[component]
pub fn Artist(id: i32) -> Element {
    rsx! { "artist {id}" }
}

#[component]
pub fn Albums() -> Element {
    rsx! { "albums" }
}

#[component]
pub fn Album(id: i32) -> Element {
    rsx! { "album {id}" }
}

#[component]
pub fn Playlists() -> Element {
    rsx! { "playlists" }
}

#[component]
pub fn Playlist(id: i32) -> Element {
    rsx! { "playlist {id}" }
}

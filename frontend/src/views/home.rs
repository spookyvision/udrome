use dioxus::prelude::*;
use dioxus_logger::tracing::debug;
use subsonic_types::response::Child;

use crate::{components::Udrome, model::SongInfo};

#[component]
pub fn Home() -> Element {
    rsx! {
        Udrome {}
    }
}

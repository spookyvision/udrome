use dioxus::prelude::*;

use crate::components::Udrome;

#[component]
pub fn Home() -> Element {
    rsx! {
        Udrome {}
    }
}

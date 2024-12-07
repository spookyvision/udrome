use dioxus::prelude::*;

use crate::components::{Hero, Udrome};

#[component]
pub fn Home() -> Element {
    rsx! {
        Udrome {}
    }
}

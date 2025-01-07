use dioxus::prelude::*;
use dioxus_logger::tracing::{debug, error};
use subsonic_types::response::Child;

use crate::{
    model::{globals::SONG, SongInfo},
    Route,
};
const NAVBAR_CSS: Asset = asset!("/assets/styling/blog.css");

#[component]
pub fn Navbar() -> Element {
    let base_url = {
        let val = option_env!("BACKEND_URL")
            .map(|e| e.to_string())
            .unwrap_or_else(|| {
                web_sys::window()
                    .map(|win| win.location().href().inspect_err(|e| error!("{e:?}")).ok())
                    .flatten()
                    .expect("could not determine origin URL")
            });

        val.strip_suffix("/").map(|s| s.to_string()).unwrap_or(val)
    };

    let cover_art_url = SONG
        .read()
        .as_ref()
        .map(|song| song.cover_art_url(&base_url))
        .flatten()
        .unwrap_or_default();

    rsx! {
        document::Stylesheet { href: NAVBAR_CSS }
        button {
            "data-drawer-toggle": "sidebar",
            r#type: "button",
            "aria-controls": "default-sidebar",
            "data-drawer-target": "default-sidebar",
            class: "inline-flex items-center p-2 mt-2 ms-3 text-sm text-gray-500 rounded-lg sm:hidden hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-gray-200 dark:text-gray-400 dark:hover:bg-gray-700 dark:focus:ring-gray-600",
            span { class: "sr-only", "Open sidebar" }
            svg {
                "aria-hidden": "true",
                fill: "currentColor",
                "viewBox": "0 0 20 20",
                xmlns: "http://www.w3.org/2000/svg",
                class: "w-6 h-6",
                path {
                    d: "M2 4.75A.75.75 0 012.75 4h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 4.75zm0 10.5a.75.75 0 01.75-.75h7.5a.75.75 0 010 1.5h-7.5a.75.75 0 01-.75-.75zM2 10a.75.75 0 01.75-.75h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 10z",
                    "clip-rule": "evenodd",
                    "fill-rule": "evenodd",
                }
            }
        }
        nav {
            "aria-label": "Sidebar",
            class: "fixed top-0 left-0 w-64 h-screen transition-transform -translate-x-full sm:translate-x-0",
            id: "sidebar",
            div { class: "h-full px-3 py-4 overflow-y-auto bg-gray-50 dark:bg-gray-800",
                ul { class: "space-y-2 font-medium",
                    li {
                        Link {
                            to: Route::Home {},
                            class: "flex items-center p-2 text-gray-900 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group",
                            span { class: "ms-3", "Home" }
                        }
                        Link {
                            to: Route::Albums {},
                            class: "flex items-center p-2 text-gray-900 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group",
                            span { class: "ms-3", "Albums" }
                        }
                        Link {
                            to: Route::Artists {},
                            class: "flex items-center p-2 text-gray-900 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group",
                            span { class: "ms-3", "Artists" }
                        }
                        Link {
                            to: Route::Playlists {},
                            class: "flex items-center p-2 text-gray-900 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group",
                            span { class: "ms-3", "Playlists" }
                        }
                    }
                }
            }
        }
        img {
            class: "fixed bottom-0 left-0 w-64 h-64 z-1 rounded border border-gray-900",
            src: "{cover_art_url}",
            onclick: move |_ev| {
                debug!("{cover_art_url}");
            },
        }


        Outlet::<Route> {}
    }
}

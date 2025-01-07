use dioxus::prelude::*;

use super::SongInfo;

pub(crate) static SONG: GlobalSignal<Option<SongInfo>> = Signal::global(|| None);

use dioxus::prelude::*;
use shrinkwraprs::Shrinkwrap;
use web_sys::HtmlAudioElement;

use super::SongInfo;

#[derive(Clone, Shrinkwrap)]
pub struct BaseUrl(pub String);

pub(crate) static SONG: GlobalSignal<Option<SongInfo>> = Signal::global(|| None);

pub(crate) static PLAYER: GlobalSignal<Option<HtmlAudioElement>> = Signal::global(|| None);

#[derive(Debug, Copy, Clone)]
pub(crate) enum Focus {
    Player,
    Search,
}

pub(crate) static FOCUS: GlobalSignal<Option<Focus>> = Signal::global(|| None);

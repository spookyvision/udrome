use std::{cell::RefCell, sync::Arc};

use async_channel::{Receiver, Sender};
use dioxus::prelude::*;
use futures::{lock::Mutex, StreamExt};
use shrinkwraprs::Shrinkwrap;
use web_sys::HtmlAudioElement;

use super::SongInfo;

#[derive(Clone, Shrinkwrap)]
pub struct BaseUrl(pub String);

pub(crate) static SONG: GlobalSignal<Option<SongInfo>> = Signal::global(|| None);

#[derive(Debug, Copy, Clone)]
pub(crate) enum Command {
    Noop,
    FocusSearch,
    BlurSearch,
}

pub(crate) static PLAYER: GlobalSignal<Option<HtmlAudioElement>> = Signal::global(|| None);

#[derive(Debug, Copy, Clone)]
pub(crate) enum Focus {
    Player,
    Search,
}

pub(crate) static FOCUS: GlobalSignal<Option<Focus>> = Signal::global(|| None);

pub(crate) type CommandBroadcast = (Sender<Command>, Receiver<Command>);

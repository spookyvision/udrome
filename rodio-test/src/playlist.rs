use id3::{Tag, TagLike};
use rodio::{source::EmptyCallback, Decoder, Sink};
use std::{
    collections::HashMap,
    io::{BufRead, Seek},
    sync::{mpsc::SyncSender, Arc, Mutex},
    time::Duration,
};
use tracing::{debug, warn};

use crate::app::Command;
type DynResult<T> = Result<T, Box<dyn std::error::Error>>;
pub struct Player {
    // TODO this does not belong here, but for now...
    metadata: HashMap<String, Box<dyn MetadataProvider>>,
    entries: Vec<String>,
    idx: Arc<Mutex<usize>>,
    valid: bool,
    backend: Sink,
    update_tx: SyncSender<Command>,
}

impl Player {
    pub fn new(backend: Sink, update_tx: SyncSender<Command>) -> Self {
        Self {
            metadata: Default::default(),
            entries: Default::default(),
            idx: Default::default(),
            valid: true,
            backend,
            update_tx,
        }
    }

    pub fn try_seek(&self, pos: Duration) -> DynResult<()> {
        self.backend.try_seek(pos).map_err(|e| e.into())
    }

    pub fn get_pos(&self) -> Duration {
        self.backend.get_pos()
    }

    pub fn cur(&self) -> Option<&dyn MetadataProvider> {
        let idx = self.idx.lock().unwrap();
        let Some(cur) = self.entries.get(*idx) else {
            return None;
        };
        self.metadata.get(cur).map(|inner| inner.as_ref())
    }

    pub fn append<S>(&mut self, source: S) -> DynResult<()>
    where
        S: Seek + BufRead + Send + Sync + Clone + AsRef<str> + 'static,
    {
        let name = source.as_ref().to_owned();

        let decoder = Decoder::new(source.clone())?;
        self.backend.append(decoder);

        let idx = self.idx.clone();
        let update_tx = self.update_tx.clone();

        let cb: EmptyCallback<f32> = EmptyCallback::new(Box::new(move || {
            let mut idx = idx.lock().unwrap();
            *idx += 1;
            // TODO does this race? (callback finishes before backend starts playing next item)
            update_tx.send(Command::Update);
            warn!("TODO invalidate/sync");
            // if !self.valid {
            //     self.synchronize_with_backend();
            // }
        }));

        self.backend.append(cb);

        let tag = Tag::read_from2(source).unwrap_or_default();
        debug!(
            "resolved {name} to {} - {} ({})",
            MetadataProvider::artist(&tag),
            MetadataProvider::title(&tag),
            tag.length().as_secs()
        );
        self.metadata.insert(name.clone(), Box::new(tag));
        self.entries.push(name);

        Ok(())
    }

    pub fn synchronize_with_backend(&self) {}

    pub fn next(&self) {
        self.backend.skip_one()
    }
}

pub trait MetadataProvider {
    fn artist(&self) -> &str;
    fn title(&self) -> &str;
    fn info(&self) -> String;
    fn length(&self) -> Duration;
}

impl MetadataProvider for Tag {
    fn artist(&self) -> &str {
        TagLike::artist(self).unwrap_or_default()
    }

    fn title(&self) -> &str {
        TagLike::title(self).unwrap_or_default()
    }

    fn length(&self) -> Duration {
        let secs_u32 = self.duration().unwrap_or_default();
        Duration::from_secs(secs_u32 as _)
    }

    fn info(&self) -> String {
        format!(
            "{} - {}",
            MetadataProvider::artist(self),
            MetadataProvider::title(self)
        )
    }
}

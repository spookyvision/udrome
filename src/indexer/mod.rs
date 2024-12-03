use std::{
    collections::HashMap,
    num::NonZero,
    sync::{Arc, RwLock},
};

use camino::{Utf8Path, Utf8PathBuf};
use id3::{Tag, TagLike};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio::{
    spawn,
    sync::mpsc::{self, Sender},
};
use tracing::{debug, error, info, trace, warn};

use crate::{load, options::Args, FileVisitor};

#[derive(Debug, Clone)]
pub struct Metadata {
    pub tag: Option<Tag>,
}

impl From<Tag> for Metadata {
    fn from(tag: Tag) -> Self {
        Self { tag: Some(tag) }
    }
}

pub type SongId = String;
#[derive(Debug, Default)]
pub struct DB {
    scan_entries: Arc<RwLock<HashMap<Utf8PathBuf, Metadata>>>,
    songs: Arc<RwLock<HashMap<SongId, Utf8PathBuf>>>,
}

impl DB {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn add_song(&self, id: SongId, path: Utf8PathBuf) {
        debug!("{id} => {path}");
        let mut lock = self.songs.write().expect("rwb0rk");
        lock.insert(id, path);
    }

    pub fn meta(&self, path: impl AsRef<Utf8Path>) -> Option<Metadata> {
        let lock = self.scan_entries.read().expect("r0kb");
        lock.get(path.as_ref()).cloned()
    }
    pub fn song(&self, id: &SongId) -> Option<Utf8PathBuf> {
        let lock = self.songs.read().expect("rwb0rk");
        lock.get(id).cloned()
    }

    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&Utf8PathBuf, &Metadata),
    {
        let lock = self.scan_entries.read().expect("rwb0rk");

        for (bof, met) in lock.iter() {
            f(bof, met);
        }
    }
    fn add(&self, file: Utf8PathBuf, md: Metadata) {
        match self.scan_entries.write() {
            Ok(mut eg) => {
                // info!("ja nice {file} {:?} – {:?}", tag.artist(), tag.title());
                eg.insert(file, md);
            }
            Err(e) => {
                tracing::error!("X_X");
            }
        }
    }
}

#[derive(Clone)]
struct Visitor {
    tx: Sender<Utf8PathBuf>,
}

impl FileVisitor for Visitor {
    async fn visit(&mut self, entry: impl AsRef<Utf8Path>) {
        if let Err(e) = self.tx.send(entry.as_ref().to_owned()).await {
            error!("queue error: {e:?}")
        }
    }
}
pub struct Indexer {
    root: Utf8PathBuf,
    db: Arc<DB>,
    skip_tagged: bool,
}
impl Indexer {
    pub fn new(args: &Args) -> Self {
        let media_path = Utf8Path::from_path(&args.media_path).expect("media path error");
        Indexer {
            root: media_path.to_owned(),
            skip_tagged: args.skip_tagged,
            db: Default::default(),
        }
    }
    pub fn into_db(self) -> Arc<DB> {
        let mut db = self.db;
        let mut song_id = 1;
        db.for_each(|path, _meta| {
            db.add_song(format!("s-{song_id}"), path.clone());
            song_id += 1;
        });
        db
    }
    pub async fn run(&self) {
        // SAFETY: 4 is non zero
        const DEFAULT_PAR: NonZero<usize> = unsafe { NonZero::new_unchecked(4) };
        let par = std::thread::available_parallelism().unwrap_or_else(|_| {
            warn!("unable to determine available parallelism; defaulting to {DEFAULT_PAR}");
            DEFAULT_PAR
        });
        info!("par {par}");
        let par = par.into();
        let (task_tx, mut task_rx) = mpsc::channel(par);

        let db = self.db.clone();
        spawn(async move {
            let mut entries = Vec::with_capacity(par);

            loop {
                task_rx.recv_many(&mut entries, par).await;
                entries.par_iter().for_each(|entry: &Utf8PathBuf| {
                    trace!("processing {entry}");
                    let md = match Tag::read_from_path(entry) {
                        Ok(tag) => tag.into(),
                        Err(_) => Metadata { tag: None },
                    };
                    db.add(entry.to_owned(), md);
                });
            }
        });

        let visitor = Visitor { tx: task_tx };

        let count = Default::default();
        debug!("indexer::start");
        load(&self.root, visitor, &count).await;
        debug!("indexer::finish");
    }
}

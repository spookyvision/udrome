use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
    os::unix::fs::MetadataExt,
    sync::{Arc, RwLock},
};

use camino::{Utf8Path, Utf8PathBuf};
use db::DB;
use id3::{Tag, TagLike};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sea_orm::{ActiveModelBehavior, ActiveValue as AV};
use tokio::{
    spawn,
    sync::mpsc::{self, Sender},
};
use tracing::{debug, error, info, trace, warn};

use crate::{entity::song, load, options::Args, FileVisitor};

mod migration;
#[derive(Debug, Clone)]
pub struct Metadata {
    pub tag: Option<Tag>,
}

impl Metadata {
    pub fn tag(&self) -> Option<&Tag> {
        self.tag.as_ref()
    }
}

impl From<Tag> for Metadata {
    fn from(tag: Tag) -> Self {
        Self { tag: Some(tag) }
    }
}

pub mod db;

#[derive(Clone)]
struct Visitor {
    tx: Sender<Utf8PathBuf>,
}

impl FileVisitor for Visitor {
    fn visit(
        &mut self,
        entry: impl AsRef<Utf8Path>,
    ) -> impl std::future::Future<Output = ()> + Send {
        let entry = entry.as_ref().to_owned();
        async {
            if let Err(e) = self.tx.send(entry).await {
                error!("queue error: {e:?}")
                // panic!("queue error: {e:?}")
            }
        }
    }
}

struct IndexerResult {
    path: Utf8PathBuf,
    meta: Metadata,
}

impl IndexerResult {
    fn title(&self) -> String {
        self.meta
            .tag()
            .map(|t| t.title())
            .flatten()
            .unwrap_or(self.path.file_name().expect("not a file?"))
            .to_string()
    }

    fn artist(&self) -> Option<String> {
        self.meta
            .tag()
            .map(|t| t.artist())
            .flatten()
            .map(|s| s.to_string())
    }

    fn size(&self) -> Option<u64> {
        std::fs::metadata(&self.path).ok().map(|md| md.size())
    }
}
pub struct Indexer {
    root: Utf8PathBuf,
    db: Arc<DB>,
    skip_tagged: bool,
}
impl Indexer {
    pub async fn new(args: &Args) -> Result<Self, db::Error> {
        let media_path = Utf8Path::from_path(&args.media_path).expect("media path error");
        let db_path = Utf8Path::from_path(&args.db_path).expect("db path error");
        Ok(Indexer {
            root: media_path.to_owned(),
            skip_tagged: args.skip_tagged,
            db: Arc::new(DB::new(&db_path).await?),
        })
    }
    pub fn db(&self) -> Arc<DB> {
        self.db.clone()
    }
    pub async fn run(&self) {
        // SAFETY: 4 is non zero
        const DEFAULT_PAR: NonZero<usize> = unsafe { NonZero::new_unchecked(4) };
        let par = std::thread::available_parallelism().unwrap_or_else(|_| {
            warn!("unable to determine available parallelism; defaulting to {DEFAULT_PAR}");
            DEFAULT_PAR
        });
        info!("gotta go this fast: {par}");
        let par = par.into();

        let (indexer_tx, mut indexer_rx) = mpsc::channel(par);

        // TODO assumes 100 is a good batch size for sql insertions, needs research
        // maybe better to not batch at all so we can error on row level
        let io_par = 100;
        let (db_tx, mut db_rx) = mpsc::channel(io_par);

        let db = self.db.clone();

        spawn(async move {
            let mut entries = Vec::with_capacity(io_par);

            loop {
                if db_rx.is_closed() {
                    warn!("FIXME: db channel has shut down");
                    return;
                }
                let count = db_rx.recv_many(&mut entries, io_par).await;
                if count > 0 {
                    debug!("received {count} entities, adding {}", entries.len());
                    let entities: Vec<_> = entries
                        .iter()
                        .map(|info: &IndexerResult| {
                            let mime_type = mime_guess::from_path(&info.path);
                            // TODO error handling
                            let size = info.size().map(|sz| sz.try_into().expect("seriously?"));
                            song::ActiveModel {
                                // parent: todo!(),
                                title: AV::Set(info.title()),
                                path: AV::Set(info.path.to_string()),
                                // album: todo!(),
                                artist: AV::Set(info.artist()),
                                // track: todo!(),
                                // year: todo!(),
                                // genre: todo!(),
                                // cover_art: todo!(),
                                size: AV::Set(size),
                                content_type: AV::Set(
                                    mime_type.first().map(|inner| inner.to_string()),
                                ),
                                ..Default::default()
                            }
                        })
                        .collect();
                    if let Err(e) = db.add_all(entities).await {
                        error!("updating database: {e}");
                    }
                    entries.clear();
                }
            }
        });

        spawn(async move {
            let mut entries = Vec::with_capacity(par);

            let mut quarantine = HashSet::<&str>::new();
            quarantine.extend(&["12 - Fragments of freedom.mp3"]);
            loop {
                if indexer_rx.is_closed() {
                    warn!("FIXME: indexer channel has shut down");
                    return;
                }
                indexer_rx.recv_many(&mut entries, par).await;
                // collect is wasteful but we need an async context for queue send
                let mds: Vec<_> = entries
                    .par_iter()
                    .map(|path: &Utf8PathBuf| {
                        trace!("processing {path} {:?}", path.file_name());
                        let meta = if quarantine.contains(path.file_name().expect("no file name?!"))
                        {
                            warn!("quarantined: {path}");
                            Metadata { tag: None }
                        } else {
                            match Tag::read_from_path(path) {
                                Ok(tag) => tag.into(),
                                Err(_) => Metadata { tag: None },
                            }
                        };

                        IndexerResult {
                            path: path.to_owned(),
                            meta,
                        }
                    })
                    .collect();

                for md in mds {
                    if let Err(e) = db_tx.send(md).await {
                        warn!("tx error (OK on shutdown) {e}");
                    }
                }
                entries.clear();
            }
        });

        let visitor = Visitor { tx: indexer_tx };

        let count = Default::default();
        debug!("indexer::start");
        load(&self.root, visitor, &count).await;
        debug!("indexer::finish");
    }
}

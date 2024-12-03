use std::{
    collections::HashMap,
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
            .unwrap_or(self.path.as_str())
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
    pub fn into_db(self) -> Arc<DB> {
        let mut db = self.db;
        // let mut song_id = 1;
        // db.for_each(|path, _meta| {
        //     db.add_song(format!("s-{song_id}"), path.clone());
        //     song_id += 1;
        // });
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
        let (indexer_tx, mut indexer_rx) = mpsc::channel(par);

        // TODO assumes 100 is a good batch size for sql insertions, needs research
        let io_par = 100;
        let (db_tx, mut db_rx) = mpsc::channel(io_par);

        let db = self.db.clone();

        spawn(async move {
            let mut entries = Vec::with_capacity(io_par);

            loop {
                // TODO according to docs this shouldn't return 0 when limit (io_par) != 0
                // but it apparently does?
                let count = db_rx.recv_many(&mut entries, io_par).await;
                if count > 0 {
                    debug!("adding {count} entities");
                    let entities: Vec<_> = entries
                        .iter()
                        .map(|info: &IndexerResult| {
                            let mime_type = mime_guess::from_path(&info.path);
                            song::ActiveModel {
                                // parent: todo!(),
                                title: AV::Set(info.title()),
                                // album: todo!(),
                                artist: AV::Set(info.artist()),
                                // track: todo!(),
                                // year: todo!(),
                                // genre: todo!(),
                                // cover_art: todo!(),
                                size: AV::Set(info.size()),
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
                }
            }
        });

        spawn(async move {
            let mut entries = Vec::with_capacity(par);

            loop {
                indexer_rx.recv_many(&mut entries, par).await;

                // collect is wasteful but we need an async context for queue send
                let mds: Vec<_> = entries
                    .par_iter()
                    .map(|path: &Utf8PathBuf| {
                        trace!("processing {path}");
                        let meta = match Tag::read_from_path(path) {
                            Ok(tag) => tag.into(),
                            Err(_) => Metadata { tag: None },
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

use std::{
    collections::HashSet, num::NonZero, os::unix::fs::MetadataExt, sync::Arc, time::Duration,
};

use camino::{Utf8Path, Utf8PathBuf};
use db::DB;
use id3::{frame::Picture, Tag, TagLike};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sea_orm::{ActiveValue as AV, EntityTrait, InsertResult};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    spawn,
    sync::mpsc::{self, Sender},
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    entity::{cover_art, song},
    load,
    options::Args,
    FileVisitor,
};

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
    fn duration(&self) -> Option<Duration> {
        mp3_duration::from_path(&self.path).ok()
    }
    pub fn pictures(&self) -> Vec<&Picture> {
        self.meta
            .tag()
            .map(|t| t.pictures().collect::<Vec<_>>())
            .unwrap_or_default()
    }
}
pub struct Indexer {
    media_path: Utf8PathBuf,
    db: Arc<DB>,
    skip_tagged: bool,
}
impl Indexer {
    pub async fn new(args: &Args) -> Result<Self, db::Error> {
        let media_path = Utf8Path::from_path(&args.media_path).expect("media path error");
        let db_path = Utf8Path::from_path(&args.db_path).expect("db path error");
        Ok(Indexer {
            media_path: media_path.to_owned(),
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

        let (indexer_tx, mut indexer_rx) = mpsc::channel::<Utf8PathBuf>(par);

        // TODO assumes 100 is a good batch size for sql insertions, needs research
        // maybe better to not batch at all so we can error on row level
        let io_par = 100;
        let (db_tx, mut db_rx) = mpsc::channel::<IndexerResult>(io_par);

        let db = self.db.clone();

        let everything = self.db.all_songs().await;
        let mut known = HashSet::new();
        known.extend(everything.into_iter().map(|song| song.path));

        spawn(async move {
            let mut entries = Vec::with_capacity(io_par);

            loop {
                if db_rx.is_closed() {
                    warn!("FIXME: db channel has shut down");
                    return;
                }
                let count = db_rx.recv_many(&mut entries, io_par).await;
                for info in &entries {
                    {
                        let mime_type = mime_guess::from_path(&info.path);
                        // TODO error handling
                        let size = info.size().map(|sz| sz.try_into().expect("seriously?"));

                        // TODO transaction

                        let song = song::ActiveModel {
                            // parent: todo!(),
                            title: AV::Set(info.title()),
                            path: AV::Set(info.path.to_string()),
                            // album: todo!(),
                            artist: AV::Set(info.artist()),
                            // track: todo!(),
                            duration: AV::Set(info.duration().map(|d| d.as_secs() as u32)),
                            // year: todo!(),
                            // genre: todo!(),
                            // cover_art: todo!(),
                            size: AV::Set(size),
                            content_type: AV::Set(mime_type.first().map(|inner| inner.to_string())),
                            ..Default::default()
                        };

                        let song_id = match song::Entity::insert(song).exec(db.connection()).await {
                            Ok(inner_res) => Some(inner_res.last_insert_id),
                            Err(e) => {
                                trace!("inserting song: {e}");
                                None
                            }
                        };

                        if let Some(song_id) = song_id {
                            let pictures = info.pictures();
                            if !pictures.is_empty() {
                                let pic = pictures[0];

                                // 512 shards ought to be enough for anybody
                                let shard = (rand::random::<u32>() % 512) as _;
                                let cover_art = cover_art::ActiveModel {
                                    shard: AV::Set(shard as _),
                                    mime_type: AV::Set(pic.mime_type.clone()),
                                    song: AV::Set(song_id),
                                    ..Default::default()
                                };

                                match cover_art::Entity::insert(cover_art)
                                    .exec(db.connection())
                                    .await
                                {
                                    Ok(res) => {
                                        let data_path = db.data_path();
                                        if let Err(e) = cover_art::Model::write(
                                            &pic.data,
                                            res.last_insert_id,
                                            shard,
                                            data_path,
                                        )
                                        .await
                                        {
                                            error!("writing cover art: {e}");
                                        }
                                    }
                                    Err(e) => trace!("inserting cover art: {e}"),
                                }
                            }
                        }
                    }
                }

                entries.clear();
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
                    .filter(|entry| !known.contains(entry.as_str()))
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
        load(&self.media_path, visitor, &count).await;
        debug!("indexer::finish");
    }
}

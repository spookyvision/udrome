use std::{collections::HashSet, num::NonZero, sync::Arc, time::Duration};

use camino::{Utf8Path, Utf8PathBuf};
use db::DB;
use ffprobe::{metadata, Tag as FFProbeTag};
use filesize::PathExt;
use id3::{frame::Picture, Tag as Id3Tag, TagLike};
use mime_guess::{
    mime::{AUDIO, MPEG},
    Mime,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sea_orm::{ActiveValue as AV, EntityTrait};
use tokio::{
    spawn,
    sync::mpsc::{self, Sender},
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    config::{Config, Indexer as IndexerConfig},
    entity::{cover_art, song},
    load,
    util::{Pwn, Unpwn},
    FileVisitor,
};

mod migration;

pub mod db;

pub(crate) mod types;

mod ffprobe;

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

#[derive(Debug, Clone)]
enum Tag {
    Ffprobe(FFProbeTag),
    Id3(Id3Tag),
}

impl Tag {
    fn title(&self) -> Option<&str> {
        match self {
            Tag::Ffprobe(tag) => Some(&tag.title),
            Tag::Id3(tag) => tag.title(),
        }
    }

    fn artist(&self) -> Option<&str> {
        match self {
            Tag::Ffprobe(tag) => tag.artist.unpwn(),
            Tag::Id3(tag) => tag.artist(),
        }
    }

    fn album(&self) -> Option<&str> {
        match self {
            Tag::Ffprobe(tag) => tag.album.as_deref(),
            Tag::Id3(tag) => tag.album(),
        }
    }
}
#[derive(Debug)]
struct IndexerResult {
    path: Utf8PathBuf,
    tag: Option<Tag>,
    mime_type: Option<Mime>,
}

impl IndexerResult {
    fn title(&self) -> &str {
        self.tag
            .as_ref()
            .map(|t| t.title())
            .flatten()
            .unwrap_or(self.path.file_name().expect("not a file?"))
    }

    fn artist(&self) -> Option<&str> {
        self.tag.as_ref().map(|t| t.artist()).flatten()
    }

    fn album(&self) -> Option<&str> {
        self.tag.as_ref().map(|t| t.album()).flatten()
    }

    fn size(&self) -> Option<u64> {
        self.path.as_std_path().size_on_disk().ok()
    }
    fn duration(&self) -> Option<Duration> {
        mp3_duration::from_path(&self.path).ok()
    }
    pub fn pictures(&self) -> Vec<&Picture> {
        self.tag
            .as_ref()
            .map(|t| match t {
                Tag::Ffprobe(_) => {
                    warn!("{}: ffprobe selected for cover art extraction, but this is likely never going to get implemented", self.path);
                    vec![]
                },
                Tag::Id3(tag) => tag.pictures().collect(),
            })
            .unwrap_or_default()
    }
}
pub struct Indexer {
    media_paths: Vec<Utf8PathBuf>,
    db: Arc<DB>,
    config: IndexerConfig,
}
impl Indexer {
    pub async fn new(config: &Config) -> Result<Self, db::Error> {
        Ok(Indexer {
            media_paths: config.media.paths.clone(),
            db: Arc::new(DB::new(&config.system.data_path).await?),
            config: config.indexer.clone(),
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

        let enable = self.config.enable;
        if !enable {
            warn!("indexer disabled! (just running dirwalk)");
        }

        let (indexer_tx, mut indexer_rx) = mpsc::channel::<Utf8PathBuf>(par);

        // TODO batching is currently unused (future: can we even do batch upserts?)
        //
        // TODO assumes 100 is a good batch size for sql insertions, needs research
        // maybe better to not batch at all so we can error on row level
        let io_par = 100;
        let (db_tx, mut db_rx) = mpsc::channel::<IndexerResult>(io_par);

        let db = self.db.clone();

        let mut known = HashSet::new();
        let everything = self.db.all_songs().await;
        known.extend(everything.into_iter().map(|song| song.path));

        spawn(async move {
            let mut entries = Vec::with_capacity(io_par);

            loop {
                db_rx.recv_many(&mut entries, io_par).await;
                if db_rx.is_closed() {
                    warn!("FIXME: db channel has shut down");
                    return;
                }
                for info in &entries {
                    {
                        trace!("inserting {:?} - {}", info.artist(), info.title());

                        // TODO error handling
                        let size = info.size().map(|sz| sz.try_into().expect("seriously?"));

                        // TODO transaction

                        let song = song::ActiveModel {
                            // parent: todo!(),
                            title: AV::Set(info.title().to_string()),
                            path: AV::Set(info.path.to_string()),
                            album: AV::Set(info.album().to_pwned()),
                            artist: AV::Set(info.artist().to_pwned()),
                            // track: todo!(),
                            duration: AV::Set(info.duration().map(|d| d.as_secs() as u32)),
                            // year: todo!(),
                            // genre: todo!(),
                            // cover_art: todo!(),
                            size: AV::Set(size),
                            content_type: AV::Set(
                                info.mime_type.as_ref().map(|inner| inner.to_string()),
                            ),
                            ..Default::default()
                        };

                        let song_id = match song::Entity::insert(song).exec(db.connection()).await {
                            Ok(inner_res) => Some(inner_res.last_insert_id),
                            Err(e) => {
                                warn!("inserting song: {e}");
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

        let mut exclude_files = HashSet::<String>::new();
        exclude_files.extend(self.config.exclude.files.iter().map(|s| s.to_string()));

        spawn(async move {
            let mut entries = Vec::with_capacity(par);

            loop {
                indexer_rx.recv_many(&mut entries, par).await;
                if !enable {
                    continue;
                }
                if indexer_rx.is_closed() {
                    warn!("FIXME: indexer channel has shut down");
                    return;
                }
                // trace!("workload {}", entries.len());

                // collect is wasteful but we need an async context for queue send
                let mds: Vec<_> = entries
                    .par_iter()
                    .filter(|entry| {
                        let is_known = known.contains(entry.as_str());
                        let is_exclude =
                            exclude_files.contains(entry.file_name().expect("no file name?"));
                        if is_exclude {
                            warn!("excluding {entry}");
                        }

                        !(is_known || is_exclude)
                    })
                    .map(|path: &Utf8PathBuf| {
                        trace!("processing {path} {:?}", path.file_name());
                        let mime_type = mime_guess::from_path(path).first();

                        let tag = match mime_type.as_ref().map(|m| (m.type_(), m.subtype())) {
                            Some((AUDIO, MPEG)) => match Id3Tag::read_from_path(path) {
                                Ok(tag) => Some(Tag::Id3(tag)),
                                Err(e) if matches!(e.kind, id3::ErrorKind::NoTag) => None,
                                Err(e) => {
                                    warn!("error reading Id3: {e:?}");
                                    None
                                }
                            },
                            Some((t, s)) => {
                                info!("{path} is not an mp3: {t}/{s} - using ffprobe");
                                match metadata(path) {
                                    Ok(md) => Some(Tag::Ffprobe(md.into_tag())),
                                    // deser error means mostly either "no suitable metadata", which is ok, go `None` then
                                    // or NonUtf8, which we still need to handle
                                    Err(ffprobe::Error::Deser(e)) => {
                                        warn!("TODO handle nonUtf8 {e:?}");
                                        None
                                    }
                                    Err(e) => {
                                        warn!("metadata error: {e}");
                                        None
                                    }
                                }
                            }
                            // TODO DRY vs. Some() arm
                            None => {
                                warn!("could not determine mime type for {path}");
                                match metadata(path) {
                                    Ok(md) => Some(Tag::Ffprobe(md.into_tag())),
                                    // deser error means mostly either "no suitable metadata", which is ok, go `None` then
                                    // or NonUtf8, which we still need to handle
                                    Err(ffprobe::Error::Deser(e)) => {
                                        warn!("hmm {e:?}");
                                        None
                                    }
                                    Err(e) => {
                                        warn!("metadata error: {e}");
                                        None
                                    }
                                }
                            }
                        };

                        IndexerResult {
                            path: path.to_owned(),
                            tag,
                            mime_type,
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
        for path in &self.media_paths {
            load(path, visitor.clone(), &count).await;
        }
        debug!("indexer::finish {count:?}");
    }
}

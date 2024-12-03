use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use camino::{Utf8Path, Utf8PathBuf};
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr, EntityTrait};
use thiserror::Error;
use tracing::{debug, error, info, trace, warn};

use super::Metadata;
use crate::entity::song;
pub type SongId = String;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Db")]
    DbErr(#[from] DbErr),
    #[error("Utf8")]
    NonUtf8,
}
#[derive(Debug, Default)]
pub struct DB {
    scan_entries: Arc<RwLock<HashMap<Utf8PathBuf, Metadata>>>,
    songs: Arc<RwLock<HashMap<SongId, Utf8PathBuf>>>,
    connection: DatabaseConnection,
}

impl DB {
    pub(super) async fn new(path: impl AsRef<Utf8Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        // TODO needed?
        assert!(path.is_absolute());

        let db_url = format!("sqlite://{path}/udrome.sqlite?mode=rwc");
        debug!("database URL: {db_url}");
        let mut opts = ConnectOptions::new(db_url);
        opts.sqlx_logging(true)
            .sqlx_logging_level(log::LevelFilter::Info);

        let connection = Database::connect(opts).await?;

        Ok(Self {
            connection,
            ..Default::default()
        })
    }

    pub async fn add_all(&self, songs: Vec<song::ActiveModel>) -> Result<(), DbErr> {
        let res = song::Entity::insert_many(songs)
            .on_empty_do_nothing()
            .exec(&self.connection)
            .await?;
        // TODO these variants seem to refer to a single row, not multiple?
        match res {
            sea_orm::TryInsertResult::Empty => warn!("empty insert operation"),
            sea_orm::TryInsertResult::Conflicted => error!("conflict while inserting into db"),
            sea_orm::TryInsertResult::Inserted(_) => trace!("insert ok"),
        }
        Ok(())
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

    pub async fn query_song(&self, id: i32) -> Result<Option<song::Model>, DbErr> {
        Ok(song::Entity::find_by_id(id).one(&self.connection).await?)
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
    pub(super) fn add(&self, file: Utf8PathBuf, md: Metadata) {
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

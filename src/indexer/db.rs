use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use camino::{Utf8Path, Utf8PathBuf};
use sea_orm::{
    ConnectOptions, Database, DatabaseConnection, DbErr, EntityTrait, Order, QueryOrder,
    QuerySelect,
};
use sea_orm_migration::MigratorTrait;
use subsonic_types::request::search::Search3;
use thiserror::Error;
use tracing::{debug, error, info, trace, warn};

use super::Metadata;
use crate::{entity::song, indexer::migration};
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
        opts.sqlx_logging(false);
        //     .sqlx_logging_level(log::LevelFilter::Trace);

        let connection = Database::connect(opts).await?;
        migration::Migrator::up(&connection, None).await?;
        warn!("deleting all entries!");
        let res = song::Entity::delete_many().exec(&connection).await;
        debug!("{res:?}");

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

    pub async fn query(&self, query: &Search3) -> Vec<song::Model> {
        let mut res = vec![];
        debug!("{query:?}");
        match song::Entity::find()
            .order_by(song::Column::Id, Order::Asc)
            .limit(query.song_count.map(|sc| sc as u64))
            .offset(query.song_offset.map(|so| so as u64))
            .all(&self.connection)
            .await
        {
            Ok(ents) => res = ents,
            Err(e) => error!("{e}"),
        }
        res
    }
    pub async fn get_song(&self, id: impl AsRef<str>) -> Option<song::Model> {
        let Ok(id) = id.as_ref().parse::<i32>() else {
            return None;
        };
        song::Entity::find_by_id(id)
            .one(&self.connection)
            .await
            .inspect_err(|e| error!("{e:?}"))
            .ok()
            .flatten()
    }
}

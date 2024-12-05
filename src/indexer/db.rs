use camino::{Utf8Path, Utf8PathBuf};
use sea_orm::{
    prelude::Expr, sea_query::extension::postgres::PgExpr, ColumnTrait, Condition, ConnectOptions,
    Database, DatabaseConnection, DbErr, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect,
};
use sea_orm_migration::MigratorTrait;
use subsonic_types::request::search::Search3;
use thiserror::Error;
use tracing::{debug, error, info, trace, warn};

use crate::{
    entity::{
        cover_art,
        song::{self, Song},
    },
    indexer::migration,
};
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
    data_path: Utf8PathBuf,
    connection: DatabaseConnection,
}

impl DB {
    pub(super) async fn new(data_path: impl AsRef<Utf8Path>) -> Result<Self, Error> {
        let data_path = data_path.as_ref().to_path_buf();
        // TODO needed?
        // assert!(path.is_absolute());

        let db_url = format!("sqlite://{data_path}/udrome.sqlite?mode=rwc");
        debug!("database URL: {db_url}");
        let mut opts = ConnectOptions::new(db_url);
        opts.sqlx_logging(true)
            .sqlx_logging_level(log::LevelFilter::Trace);

        let connection = Database::connect(opts).await?;
        migration::Migrator::up(&connection, None).await?;
        let wipe = false;
        if wipe {
            warn!("deleting all entries!");
            let res = song::Entity::delete_many().exec(&connection).await;
            debug!("{res:?}");
        }

        Ok(Self {
            connection,
            data_path,
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

    pub async fn all_songs(&self) -> Vec<song::Model> {
        song::Entity::find()
            .all(&self.connection)
            .await
            .unwrap_or(vec![])
    }

    pub async fn query(&self, query: &Search3) -> Vec<song::Model> {
        let mut res = vec![];
        debug!("{query:?}");

        // what the user was actually searching for
        let user_query = query.query.replace("\"", "");

        let mut filter = Condition::all();

        let mut do_filter = false;
        for word in user_query.split(" ") {
            if !word.is_empty() {
                do_filter = true;
                filter = filter.add(song::Column::Title.contains(word));
            }
        }

        let mut op = song::Entity::find();

        if do_filter {
            op = op.filter(filter);
        }

        match op
            .limit(query.song_count.map(|sc| sc as u64))
            .offset(query.song_offset.map(|so| so as u64))
            .order_by(song::Column::Title, Order::Asc)
            .all(&self.connection)
            .await
        {
            Ok(ents) => res = ents,
            Err(e) => error!("{e}"),
        }
        res
    }

    pub async fn get_cover_art(&self, id: impl AsRef<str>) -> Option<cover_art::Model> {
        let Ok(id) = id.as_ref().parse::<i32>() else {
            return None;
        };
        cover_art::Entity::find_by_id(id)
            .one(self.connection())
            .await
            .inspect_err(|e| error!("get cover art {e:?}"))
            .ok()
            .flatten()
    }

    pub async fn get_cover_art_for_song(&self, song_id: i32) -> Option<cover_art::Model> {
        cover_art::Entity::find()
            .filter(cover_art::Column::Song.eq(song_id))
            .one(self.connection())
            .await
            .inspect_err(|e| error!("get cover art for song {e:?}"))
            .ok()
            .flatten()
    }
    pub async fn get_song(&self, id: impl AsRef<str>) -> Option<song::Model> {
        let Ok(id) = id.as_ref().parse::<i32>() else {
            return None;
        };
        let mut song = song::Entity::find_by_id(id)
            .one(self.connection())
            .await
            .inspect_err(|e| error!("get song {e:?}"))
            .ok()
            .flatten();

        // TODO some kind of join would be nice?
        if let Some(song) = song.as_mut() {
            song.cover_art = self
                .get_cover_art_for_song(song.id)
                .await
                .map(|ca| format!("{}", ca.id));
        }
        song
    }

    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub fn data_path(&self) -> &Utf8Path {
        self.data_path.as_path()
    }
}

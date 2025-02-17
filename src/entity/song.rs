use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::cover_art;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "song")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i32,
    // TODO can we use (Utf8)PathBuf?
    pub path: String,
    pub parent: Option<String>,
    pub title: String,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub track: Option<u32>,
    pub duration: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub cover_art: Option<String>,
    pub size: Option<u32>,
    pub content_type: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "cover_art::Entity")]
    CoverArt,
}

impl Related<cover_art::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CoverArt.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(DeriveIden)]
pub(crate) enum Song {
    Table,
    Id,
    Path,
    Parent,
    Title,
    Album,
    Artist,
    Track,
    Duration,
    Year,
    Genre,
    CoverArt,
    Size,
    ContentType,
}

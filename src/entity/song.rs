use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

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
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub cover_art: Option<String>,
    pub size: Option<u32>,
    pub content_type: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

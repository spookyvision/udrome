use camino::{Utf8Path, Utf8PathBuf};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::trace;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cover_art")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i32,
    pub shard: i32,
    pub mime_type: String,
    pub song: i32,
}

impl Model {
    fn path_inner(id: i32, shard: i32, root: &Utf8Path) -> Utf8PathBuf {
        let mut path = root.join("data");
        path.push("artwork");
        path.push(format!("{shard}"));
        path.push(format!("{id}"));
        path
    }

    pub fn path(&self, root: &Utf8Path) -> Utf8PathBuf {
        Self::path_inner(self.id, self.shard, root)
    }

    pub async fn write(data: &[u8], id: i32, shard: i32, root: &Utf8Path) -> std::io::Result<()> {
        let path = Self::path_inner(id, shard, root);
        let containing_dir = path.parent().expect("could not create containing path");
        tokio::fs::create_dir_all(containing_dir).await?;
        trace!("write {path}");
        let mut file = File::create(path).await?;
        file.write_all(data).await?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::song::Entity",
        from = "Column::Song",
        to = "super::song::Column::Id"
    )]
    Song,
}

impl Related<super::song::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Song.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

use camino::{Utf8Path, Utf8PathBuf};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::debug;

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
    pub fn path2(id: i32, shard: i32, root: &Utf8Path) -> Utf8PathBuf {
        let mut path = root.join("data");
        path.push("artwork");
        path.push(format!("{shard}"));
        path.push(format!("{id}"));
        debug!("cover path {path}");
        path
    }

    pub fn path(&self, root: &Utf8Path) -> Utf8PathBuf {
        Self::path2(self.id, self.shard, root)
    }

    pub async fn write(data: &[u8], id: i32, shard: i32, root: &Utf8Path) -> std::io::Result<()> {
        let path = Self::path2(id, shard, root);
        let containing_dir = path.parent().expect("could not create containing path");
        tokio::fs::create_dir_all(containing_dir).await?;
        debug!("write {path}");
        let mut file = File::create(path).await?;
        file.write_all(data).await?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

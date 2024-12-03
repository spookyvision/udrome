use sea_orm_migration::{prelude::*, schema::*};

use super::m20220101_000001_create_table::Song;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .create_table(
                Table::create()
                    .table(CoverArt::Table)
                    .if_not_exists()
                    .col(pk_auto(CoverArt::Id))
                    .col(integer(CoverArt::Shard))
                    .col(string(CoverArt::MimeType))
                    .col(integer(CoverArt::Song))
                    .foreign_key(
                        ForeignKey::create()
                            .from(CoverArt::Table, CoverArt::Song)
                            .to(Song::Table, Song::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .drop_table(Table::drop().table(CoverArt::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CoverArt {
    Table,
    Id,
    Shard,
    MimeType,
    Song,
}

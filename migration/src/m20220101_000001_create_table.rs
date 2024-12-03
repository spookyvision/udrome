use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Song::Table)
                    .if_not_exists()
                    .col(pk_auto(Song::Id))
                    .col(string_null(Song::Parent))
                    .col(string(Song::Title))
                    .col(string_null(Song::Album))
                    .col(string_null(Song::Artist))
                    .col(integer_null(Song::Track))
                    .col(integer_null(Song::Year))
                    .col(string_null(Song::Genre))
                    .col(string_null(Song::CoverArt))
                    .col(big_integer_null(Song::Size))
                    .col(string_null(Song::ContentType))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Song::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Song {
    Table,
    Id,
    Parent,
    Title,
    Album,
    Artist,
    Track,
    Year,
    Genre,
    CoverArt,
    Size,
    ContentType,
}

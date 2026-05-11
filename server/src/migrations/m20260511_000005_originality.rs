//! Originality score on games. Additive, idempotent. Default 0 so all rows
//! pre-dating phase 2 read as "no originality info".

use sea_orm_migration::{async_trait, prelude::*, schema::*};

use super::m20260510_000003_games::Games;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Games::Table)
                    .add_column_if_not_exists(integer(Originality::OriginalityScore).default(0))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_games_grid_originality")
                    .table(Games::Table)
                    .col(Games::GridId)
                    .col(Originality::OriginalityScore)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_games_grid_originality")
                    .table(Games::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Games::Table)
                    .drop_column(Originality::OriginalityScore)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Originality {
    OriginalityScore,
}

//! Games table — one row per (grid, device).

use sea_orm_migration::{async_trait, prelude::*, schema::*};

use super::m20260510_000001_users_devices_sessions::{Devices, Users};
use super::m20260510_000002_domains_grids::Grids;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Games::Table)
                    .if_not_exists()
                    .col(uuid(Games::Id).primary_key())
                    .col(uuid(Games::GridId))
                    .col(uuid(Games::DeviceId))
                    .col(uuid_null(Games::UserId))
                    .col(timestamp_with_time_zone(Games::StartedAt))
                    .col(timestamp_with_time_zone_null(Games::FinishedAt))
                    .col(integer(Games::Score).default(0))
                    .col(integer(Games::MaxScore).default(0))
                    .col(integer(Games::Mistakes).default(0))
                    .col(integer(Games::Solved).default(0))
                    .col(string(Games::Status).default("active"))
                    .col(json_binary(Games::Answers))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Games::Table, Games::GridId)
                            .to(Grids::Table, Grids::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Games::Table, Games::DeviceId)
                            .to(Devices::Table, Devices::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Games::Table, Games::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uniq_games_grid_device")
                    .table(Games::Table)
                    .col(Games::GridId)
                    .col(Games::DeviceId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_games_grid_score")
                    .table(Games::Table)
                    .col(Games::GridId)
                    .col(Games::Score)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Games::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum Games {
    Table,
    Id,
    GridId,
    DeviceId,
    UserId,
    StartedAt,
    FinishedAt,
    Score,
    MaxScore,
    Mistakes,
    Solved,
    Status,
    Answers,
}

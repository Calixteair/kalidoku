//! Friendships and duels.

use sea_orm_migration::{async_trait, prelude::*, schema::*};

use super::m20260510_000001_users_devices_sessions::Users;
use super::m20260510_000002_domains_grids::Grids;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Friendships::Table)
                    .if_not_exists()
                    .col(uuid(Friendships::UserA))
                    .col(uuid(Friendships::UserB))
                    .col(string(Friendships::Status).default("pending"))
                    .col(timestamp_with_time_zone(Friendships::CreatedAt))
                    .primary_key(
                        Index::create()
                            .col(Friendships::UserA)
                            .col(Friendships::UserB),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Friendships::Table, Friendships::UserA)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Friendships::Table, Friendships::UserB)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Duels::Table)
                    .if_not_exists()
                    .col(uuid(Duels::Id).primary_key())
                    .col(uuid(Duels::GridId))
                    .col(uuid_null(Duels::OwnerUserId))
                    .col(binary(Duels::ShareSig))
                    .col(timestamp_with_time_zone(Duels::ExpiresAt))
                    .col(timestamp_with_time_zone(Duels::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Duels::Table, Duels::GridId)
                            .to(Grids::Table, Grids::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Duels::Table, Duels::OwnerUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Duels::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Friendships::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum Friendships {
    Table,
    UserA,
    UserB,
    Status,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum Duels {
    Table,
    Id,
    GridId,
    OwnerUserId,
    ShareSig,
    ExpiresAt,
    CreatedAt,
}

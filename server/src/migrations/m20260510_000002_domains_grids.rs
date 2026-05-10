//! Domains catalogue and pre-generated grids.

use sea_orm_migration::{async_trait, prelude::*, schema::*};

use super::m20260510_000001_users_devices_sessions::{Devices, Users};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Domains::Table)
                    .if_not_exists()
                    .col(string(Domains::Id).primary_key())
                    .col(string(Domains::Version))
                    .col(boolean(Domains::Active).default(true))
                    .col(json_binary(Domains::Metadata))
                    .col(timestamp_with_time_zone(Domains::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Grids::Table)
                    .if_not_exists()
                    .col(uuid(Grids::Id).primary_key())
                    .col(string(Grids::Domain))
                    .col(string(Grids::Mode))
                    .col(timestamp_with_time_zone(Grids::PublishAt))
                    .col(json_binary(Grids::Payload))
                    .col(big_integer(Grids::Seed))
                    .col(timestamp_with_time_zone(Grids::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Grids::Table, Grids::Domain)
                            .to(Domains::Table, Domains::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uniq_grids_domain_mode_publish")
                    .table(Grids::Table)
                    .col(Grids::Domain)
                    .col(Grids::Mode)
                    .col(Grids::PublishAt)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Foreign keys defined in migration 0001 between Devices and Users are already in place;
        // this `use` import keeps the migration order explicit.
        let _ = (Users::Table, Devices::Table);

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Grids::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Domains::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum Domains {
    Table,
    Id,
    Version,
    Active,
    Metadata,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum Grids {
    Table,
    Id,
    Domain,
    Mode,
    PublishAt,
    Payload,
    Seed,
    CreatedAt,
}

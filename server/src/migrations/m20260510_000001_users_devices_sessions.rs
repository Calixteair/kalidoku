//! Initial tables: users, devices, sessions.

use sea_orm_migration::{async_trait, prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(uuid(Users::Id).primary_key())
                    .col(string(Users::KcSub).unique_key())
                    .col(string(Users::Pseudo).unique_key())
                    .col(string_null(Users::Email))
                    .col(string(Users::Locale).default("fr"))
                    .col(string(Users::Role).default("user"))
                    .col(boolean(Users::PremiumActive).default(false))
                    .col(timestamp_with_time_zone(Users::CreatedAt))
                    .col(timestamp_with_time_zone_null(Users::DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Devices::Table)
                    .if_not_exists()
                    .col(uuid(Devices::Id).primary_key())
                    .col(uuid_null(Devices::UserId))
                    .col(string_null(Devices::Ua))
                    .col(string_null(Devices::IpFirst))
                    .col(timestamp_with_time_zone(Devices::LastSeen))
                    .col(timestamp_with_time_zone(Devices::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Devices::Table, Devices::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(binary(Sessions::TokenHash).primary_key())
                    .col(uuid(Sessions::DeviceId))
                    .col(uuid_null(Sessions::UserId))
                    .col(timestamp_with_time_zone(Sessions::ExpiresAt))
                    .col(timestamp_with_time_zone(Sessions::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Sessions::Table, Sessions::DeviceId)
                            .to(Devices::Table, Devices::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Sessions::Table, Sessions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_user")
                    .table(Sessions::Table)
                    .col(Sessions::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Devices::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum Users {
    Table,
    Id,
    KcSub,
    Pseudo,
    Email,
    Locale,
    Role,
    PremiumActive,
    CreatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
pub enum Devices {
    Table,
    Id,
    UserId,
    Ua,
    IpFirst,
    LastSeen,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum Sessions {
    Table,
    TokenHash,
    DeviceId,
    UserId,
    ExpiresAt,
    CreatedAt,
}

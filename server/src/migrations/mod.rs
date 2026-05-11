//! SeaORM migrations runner. Owned by agent C.
//!
//! All migrations are additive and idempotent (`if not exists`).
//! Run from `main` at boot via [`Migrator::up`] when `RUN_MIGRATIONS=1`.

use sea_orm_migration::prelude::*;

mod m20260510_000001_users_devices_sessions;
mod m20260510_000002_domains_grids;
mod m20260510_000003_games;
mod m20260510_000004_friendships_duels;
mod m20260511_000005_originality;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260510_000001_users_devices_sessions::Migration),
            Box::new(m20260510_000002_domains_grids::Migration),
            Box::new(m20260510_000003_games::Migration),
            Box::new(m20260510_000004_friendships_duels::Migration),
            Box::new(m20260511_000005_originality::Migration),
        ]
    }
}

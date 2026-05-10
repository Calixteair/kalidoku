//! SeaORM database pool builder.

use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

/// Build the application's database connection pool.
pub async fn connect(database_url: &str) -> Result<DatabaseConnection> {
    let mut opts = ConnectOptions::new(database_url.to_owned());
    opts.max_connections(20)
        .min_connections(2)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(60))
        .max_lifetime(Duration::from_secs(60 * 30))
        .sqlx_logging(false);
    Database::connect(opts)
        .await
        .context("connecting to Postgres")
}

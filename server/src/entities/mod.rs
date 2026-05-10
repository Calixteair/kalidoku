//! Hand-written SeaORM entities. They mirror the migration schema in `migrations/`.
//! Kept lean: agent C writes only what the server actually queries today.

pub mod devices;
pub mod domains;
pub mod duels;
pub mod games;
pub mod grids;
pub mod sessions;
pub mod users;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "games")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub grid_id: Uuid,
    pub device_id: Uuid,
    pub user_id: Option<Uuid>,
    pub started_at: ChronoDateTimeWithTimeZone,
    pub finished_at: Option<ChronoDateTimeWithTimeZone>,
    pub score: i32,
    pub max_score: i32,
    pub mistakes: i32,
    pub solved: i32,
    pub status: String,
    pub answers: Json,
    #[sea_orm(default_value = 0)]
    pub originality_score: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

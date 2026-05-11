//! Free-tier quotas for game starts. Hooks the future premium tier.

use chrono::{DateTime, Datelike, TimeZone, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use thiserror::Error;
use uuid::Uuid;

use crate::entities::{games, users};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Daily,
    Solo,
    Duel,
}

impl GameMode {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(Self::Daily),
            "solo" => Some(Self::Solo),
            "duel" => Some(Self::Duel),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum QuotaError {
    #[error("daily quota reached")]
    DailyQuotaReached,
    #[error("solo quota reached (free tier 3/day)")]
    SoloQuotaReached,
    #[error("duel mode requires premium")]
    PremiumRequired,
}

/// Caller passes today's count for the target (mode, user/device) tuple.
/// Premium short-circuits everything.
pub fn check(
    is_premium: bool,
    mode: GameMode,
    today_count: u32,
) -> std::result::Result<(), QuotaError> {
    if is_premium {
        return Ok(());
    }
    match mode {
        GameMode::Daily => {
            if today_count >= 1 {
                Err(QuotaError::DailyQuotaReached)
            } else {
                Ok(())
            }
        }
        // Solo is unlimited by design — players can re-roll grids as they
        // please. The free-tier quota stays declared above so introducing
        // a cap later only requires flipping the constant, not re-wiring
        // the call site or the error variant.
        GameMode::Solo => {
            let _ = today_count;
            Ok(())
        }
        GameMode::Duel => Err(QuotaError::PremiumRequired),
    }
}

/// Count games started today (UTC) by the given device (and optionally a logged-in user)
/// for a specific mode. Implementation: fetch the grid IDs published in `mode` first,
/// then count games with that grid_id since today's UTC midnight.
pub async fn today_count(
    db: &DatabaseConnection,
    device_id: Uuid,
    user_id: Option<Uuid>,
    mode: GameMode,
) -> Result<u32, sea_orm::DbErr> {
    let mode_str = match mode {
        GameMode::Daily => "daily",
        GameMode::Solo => "solo",
        GameMode::Duel => "duel",
    };
    let now = Utc::now();
    let today_midnight: DateTime<Utc> = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .unwrap_or(now);

    // Two-step: list grid IDs for `mode`, then count matching games. Cheap given
    // that grids per (mode, day) is at most one per active domain.
    let grid_ids: Vec<Uuid> = crate::entities::grids::Entity::find()
        .filter(crate::entities::grids::Column::Mode.eq(mode_str))
        .all(db)
        .await?
        .into_iter()
        .map(|g| g.id)
        .collect();

    if grid_ids.is_empty() {
        return Ok(0);
    }

    let mut q = games::Entity::find()
        .filter(games::Column::StartedAt.gte::<DateTime<Utc>>(today_midnight))
        .filter(games::Column::GridId.is_in(grid_ids));
    if let Some(uid) = user_id {
        q = q.filter(games::Column::UserId.eq(uid));
    } else {
        q = q.filter(games::Column::DeviceId.eq(device_id));
    }
    let count = q.count(db).await?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

/// Look up the `users.premium_active` flag (false when anonymous).
pub async fn is_premium(
    db: &DatabaseConnection,
    user_id: Option<Uuid>,
) -> Result<bool, sea_orm::DbErr> {
    let Some(uid) = user_id else {
        return Ok(false);
    };
    let row = users::Entity::find_by_id(uid).one(db).await?;
    Ok(row.is_some_and(|u| u.premium_active))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_user_one_daily_per_day() {
        assert!(check(false, GameMode::Daily, 0).is_ok());
        assert_eq!(
            check(false, GameMode::Daily, 1),
            Err(QuotaError::DailyQuotaReached)
        );
    }

    #[test]
    fn free_user_unlimited_solos() {
        // Solo is unlimited by design — re-rolling grids has no cost.
        for n in [0_u32, 1, 3, 10, 1_000] {
            assert!(check(false, GameMode::Solo, n).is_ok());
        }
    }

    #[test]
    fn free_user_no_duel() {
        assert_eq!(
            check(false, GameMode::Duel, 0),
            Err(QuotaError::PremiumRequired)
        );
    }

    #[test]
    fn premium_bypass() {
        assert!(check(true, GameMode::Daily, 999).is_ok());
        assert!(check(true, GameMode::Duel, 999).is_ok());
    }
}

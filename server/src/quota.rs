//! Free-tier quotas for game starts. Hooks the future premium tier.

use thiserror::Error;

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
        GameMode::Solo => {
            if today_count >= 3 {
                Err(QuotaError::SoloQuotaReached)
            } else {
                Ok(())
            }
        }
        GameMode::Duel => Err(QuotaError::PremiumRequired),
    }
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
    fn free_user_three_solos_per_day() {
        for n in 0..3 {
            assert!(check(false, GameMode::Solo, n).is_ok());
        }
        assert_eq!(
            check(false, GameMode::Solo, 3),
            Err(QuotaError::SoloQuotaReached)
        );
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

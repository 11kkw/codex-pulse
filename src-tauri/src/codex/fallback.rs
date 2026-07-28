use chrono::Utc;

use crate::model::CodexSnapshot;

pub fn snapshot(reason: impl Into<String>) -> CodexSnapshot {
    CodexSnapshot {
        plan: None,
        email: None,
        connection_state: "fallback".into(),
        source: "unavailable".into(),
        status_message: reason.into(),
        primary_limit: None,
        weekly_limit: None,
        lifetime_tokens: None,
        peak_daily_tokens: None,
        longest_running_turn_seconds: None,
        current_streak_days: None,
        longest_streak_days: None,
        daily_usage: Vec::new(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

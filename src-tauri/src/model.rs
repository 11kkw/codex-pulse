use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub label: String,
    pub remaining_percent: f64,
    pub used_percent: f64,
    pub window_duration_minutes: Option<u64>,
    pub resets_at: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub tokens: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSnapshot {
    pub plan: Option<String>,
    pub email: Option<String>,
    pub connection_state: String,
    pub source: String,
    pub status_message: String,
    pub primary_limit: Option<RateLimit>,
    pub weekly_limit: Option<RateLimit>,
    pub lifetime_tokens: Option<u64>,
    pub peak_daily_tokens: Option<u64>,
    pub longest_running_turn_seconds: Option<u64>,
    pub current_streak_days: Option<u64>,
    pub longest_streak_days: Option<u64>,
    pub daily_usage: Vec<DailyUsage>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshot {
    pub available: bool,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub used_memory_bytes: u64,
    pub total_memory_bytes: u64,
    pub cpu_frequency_mhz: Option<u64>,
    pub cpu_history: Vec<f32>,
    pub memory_history: Vec<f32>,
    pub updated_at: String,
}

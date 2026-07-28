use chrono::{TimeZone, Utc};
use serde_json::Value;

use crate::model::{CodexSnapshot, DailyUsage, RateLimit};

use super::{app_server::AppServerClient, fallback};

pub struct CodexProvider {
    client: Option<AppServerClient>,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self { client: None }
    }

    pub fn snapshot(&mut self) -> CodexSnapshot {
        match self.read_live() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.client = None;
                fallback::snapshot(error)
            }
        }
    }

    fn read_live(&mut self) -> Result<CodexSnapshot, String> {
        if self.client.is_none() {
            self.client = Some(AppServerClient::connect()?);
        }
        let client = self.client.as_mut().expect("client initialized");

        let account = client
            .request_with_params("account/read", serde_json::json!({ "refreshToken": false }))?;
        let account_value = account
            .get("account")
            .filter(|value| !value.is_null())
            .ok_or_else(|| "Codex에 로그인되어 있지 않아 데모 데이터를 표시합니다.".to_string())?;
        let account_type = account_value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if account_type != "chatgpt" {
            return Err("ChatGPT 구독 로그인이 아니어서 데모 데이터를 표시합니다.".into());
        }

        let limits = client.request("account/rateLimits/read")?;
        let usage = client
            .request("account/usage/read")
            .unwrap_or_else(|_| Value::Null);

        let rate_root = limits
            .get("rateLimits")
            .filter(|value| !value.is_null())
            .or_else(|| {
                limits
                    .get("rateLimitsByLimitId")
                    .and_then(Value::as_object)
                    .and_then(|buckets| buckets.values().next())
            })
            .ok_or_else(|| "Codex 사용 한도 응답이 비어 있습니다.".to_string())?;

        let mut parsed_limits = [
            rate_root
                .get("primary")
                .filter(|value| !value.is_null())
                .and_then(|value| parse_limit(Some(value)).ok()),
            rate_root
                .get("secondary")
                .filter(|value| !value.is_null())
                .and_then(|value| parse_limit(Some(value)).ok()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        parsed_limits.sort_by_key(|limit| limit.window_duration_minutes.unwrap_or(u64::MAX));
        parsed_limits.dedup_by(|left, right| {
            left.window_duration_minutes == right.window_duration_minutes
                && left.resets_at == right.resets_at
        });
        let primary_limit = parsed_limits
            .first()
            .cloned()
            .ok_or_else(|| "Codex 사용 한도 값이 없습니다.".to_string())?;
        let weekly_limit = parsed_limits.get(1).cloned();

        let summary = usage.get("summary").unwrap_or(&Value::Null);
        let mut daily_usage: Vec<DailyUsage> = usage
            .get("dailyUsageBuckets")
            .and_then(Value::as_array)
            .map(|buckets| {
                buckets
                    .iter()
                    .filter_map(|bucket| {
                        Some(DailyUsage {
                            date: bucket.get("startDate")?.as_str()?.to_string(),
                            tokens: bucket.get("tokens")?.as_u64()?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        daily_usage.sort_by(|left, right| left.date.cmp(&right.date));

        Ok(CodexSnapshot {
            plan: account_value
                .get("planType")
                .and_then(Value::as_str)
                .map(plan_display_name),
            email: account_value
                .get("email")
                .and_then(Value::as_str)
                .map(str::to_string),
            connection_state: "connected".into(),
            source: "codex-app-server".into(),
            status_message: "Codex app-server 연결됨".into(),
            primary_limit: Some(primary_limit),
            weekly_limit,
            lifetime_tokens: summary.get("lifetimeTokens").and_then(Value::as_u64),
            peak_daily_tokens: summary.get("peakDailyTokens").and_then(Value::as_u64),
            longest_running_turn_seconds: summary
                .get("longestRunningTurnSec")
                .and_then(Value::as_u64),
            current_streak_days: summary.get("currentStreakDays").and_then(Value::as_u64),
            longest_streak_days: summary.get("longestStreakDays").and_then(Value::as_u64),
            daily_usage,
            updated_at: Utc::now().to_rfc3339(),
        })
    }
}

fn parse_limit(value: Option<&Value>) -> Result<RateLimit, String> {
    let value = value.ok_or_else(|| "Codex 사용 한도 값이 없습니다.".to_string())?;
    let used = value
        .get("usedPercent")
        .and_then(Value::as_f64)
        .ok_or_else(|| "Codex 사용 한도 비율이 없습니다.".to_string())?
        .clamp(0.0, 100.0);
    let resets_at = value
        .get("resetsAt")
        .and_then(Value::as_i64)
        .and_then(|timestamp| Utc.timestamp_opt(timestamp, 0).single())
        .map(|date| date.to_rfc3339());

    let window_duration_minutes = value.get("windowDurationMins").and_then(Value::as_u64);

    Ok(RateLimit {
        label: limit_label(window_duration_minutes),
        remaining_percent: 100.0 - used,
        used_percent: used,
        window_duration_minutes,
        resets_at,
    })
}

fn limit_label(window_duration_minutes: Option<u64>) -> String {
    match window_duration_minutes {
        Some(300) => "5시간 한도".into(),
        Some(10_080) => "주간 한도".into(),
        Some(minutes) if minutes % 1_440 == 0 => {
            format!("{}일 한도", minutes / 1_440)
        }
        Some(minutes) if minutes % 60 == 0 => {
            format!("{}시간 한도", minutes / 60)
        }
        _ => "Codex 사용 한도".into(),
    }
}

fn plan_display_name(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "free" => "Free".into(),
        "go" => "Go".into(),
        "plus" => "Plus".into(),
        "pro" => "Pro".into(),
        "prolite" => "Pro Lite".into(),
        "team" => "Team".into(),
        "self_serve_business_usage_based" => "Self Serve Business Usage Based".into(),
        "business" => "Business".into(),
        "ent26" | "enterprise" | "hc" => "Enterprise".into(),
        "enterprise_cbp_usage_based" => "Enterprise CBP Usage Based".into(),
        "education" | "edu" => "Edu".into(),
        _ => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::plan_display_name;

    #[test]
    fn uses_codex_plan_display_names() {
        assert_eq!(plan_display_name("prolite"), "Pro Lite");
        assert_eq!(plan_display_name("PLUS"), "Plus");
        assert_eq!(plan_display_name("hc"), "Enterprise");
        assert_eq!(plan_display_name("future_plan"), "future_plan");
    }
}

use std::{collections::HashSet, fs, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::{plugin::PermissionState, AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::model::CodexSnapshot;

#[derive(Default, Deserialize, Serialize)]
struct AlertHistory {
    sent: HashSet<String>,
}

pub struct AlertTracker {
    history_path: Option<PathBuf>,
    history: AlertHistory,
}

impl AlertTracker {
    pub fn load(app: &AppHandle) -> Self {
        let history_path = app
            .path()
            .app_local_data_dir()
            .ok()
            .map(|directory| directory.join("alert-history.json"));
        let history = history_path
            .as_ref()
            .and_then(|path| fs::read(path).ok())
            .and_then(|contents| serde_json::from_slice(&contents).ok())
            .unwrap_or_default();
        Self {
            history_path,
            history,
        }
    }

    pub fn evaluate(&mut self, app: &AppHandle, snapshot: &CodexSnapshot) {
        if snapshot.connection_state != "connected" {
            return;
        }

        if let Some(limit) = snapshot.weekly_limit.as_ref() {
            if let Some(cycle) = limit.resets_at.as_deref() {
                let alert = if limit.remaining_percent <= 10.0 {
                    Some((
                        format!("weekly-10:{cycle}"),
                        "Codex 주간 한도가 10% 이하로 남았습니다.",
                    ))
                } else if limit.remaining_percent <= 20.0 {
                    Some((
                        format!("weekly-20:{cycle}"),
                        "Codex 주간 한도가 20% 이하로 남았습니다.",
                    ))
                } else {
                    None
                };
                if let Some((key, body)) = alert {
                    self.notify_once(app, key, body);
                }
            }
        }

        if let Some(reset_at) = snapshot
            .primary_limit
            .as_ref()
            .and_then(|limit| limit.resets_at.as_deref())
        {
            if let Ok(reset_time) = DateTime::parse_from_rfc3339(reset_at) {
                let remaining = reset_time.with_timezone(&Utc) - Utc::now();
                if remaining.num_seconds() > 0 && remaining.num_seconds() <= 3_600 {
                    self.notify_once(
                        app,
                        format!("reset-hour:{reset_at}"),
                        "Codex 사용 한도 초기화까지 1시간 이내입니다.",
                    );
                }
            }
        }
    }

    fn notify_once(&mut self, app: &AppHandle, key: String, body: &str) {
        if self.history.sent.contains(&key) || !notifications_allowed(app) {
            return;
        }
        match app
            .notification()
            .builder()
            .title("Codex Pulse")
            .body(body)
            .show()
        {
            Ok(()) => {
                self.history.sent.insert(key);
                self.save();
            }
            Err(error) => crate::logging::write(format!("notification failed: {error}")),
        }
    }

    fn save(&self) {
        let Some(path) = self.history_path.as_ref() else {
            return;
        };
        if let Some(directory) = path.parent() {
            let _ = fs::create_dir_all(directory);
        }
        if let Ok(contents) = serde_json::to_vec_pretty(&self.history) {
            let _ = fs::write(path, contents);
        }
    }
}

pub fn ensure_permission(app: &AppHandle) {
    let notifications = app.notification();
    if matches!(
        notifications.permission_state(),
        Ok(PermissionState::Prompt)
    ) {
        let _ = notifications.request_permission();
    }
}

fn notifications_allowed(app: &AppHandle) -> bool {
    matches!(
        app.notification().permission_state(),
        Ok(PermissionState::Granted)
    )
}

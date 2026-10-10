//! Process-wide state managed by Tauri.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use serde::Serialize;

use crate::audio::AudioPlayer;
use crate::db::{repo, Db};
use crate::domain::{ActiveAlert, Clock, Task};
use crate::error::{AppError, AppResult};
use crate::scheduler::SchedulerHandle;
use crate::settings::{Settings, WindowGeometry};

/// Everything that needs a working database.
pub struct Core {
    pub db: Arc<Db>,
    pub scheduler: SchedulerHandle,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub code: String,
    pub message: String,
}

#[derive(Default)]
pub struct GeometryState {
    pub current: Option<WindowGeometry>,
    pub last_saved: Option<Instant>,
    pub dirty: bool,
}

pub struct AppState {
    pub core: RwLock<Option<Arc<Core>>>,
    pub startup_error: Mutex<Option<String>>,
    pub settings: Arc<RwLock<Settings>>,
    pub issues: Mutex<Vec<Issue>>,
    pub audio: AudioPlayer,
    pub clock: Arc<dyn Clock>,
    pub db_path: PathBuf,
    pub geometry: Mutex<GeometryState>,
    pub product_name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub product_name: String,
    pub version: String,
    pub tasks: Vec<Task>,
    pub alerts: Vec<ActiveAlert>,
    pub settings: Settings,
    pub issues: Vec<Issue>,
    pub db_path: String,
    pub startup_error: Option<String>,
    pub now_utc: i64,
}

impl AppState {
    pub fn core(&self) -> AppResult<Arc<Core>> {
        self.core
            .read()
            .map_err(|_| AppError::Internal("state lock poisoned".into()))?
            .clone()
            .ok_or_else(|| AppError::Database("Task storage is not available. Use Retry in the widget.".into()))
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn now(&self) -> i64 {
        self.clock.now_ms()
    }

    /// Record an issue once per code (most recent message wins). Returns true if new.
    pub fn add_issue(&self, code: &str, message: impl Into<String>) -> bool {
        let message = message.into();
        let Ok(mut issues) = self.issues.lock() else { return false };
        if let Some(existing) = issues.iter_mut().find(|i| i.code == code) {
            existing.message = message;
            false
        } else {
            issues.push(Issue { code: code.into(), message });
            true
        }
    }

    pub fn clear_issue(&self, code: &str) {
        if let Ok(mut issues) = self.issues.lock() {
            issues.retain(|i| i.code != code);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let (tasks, alerts) = match self.core() {
            Ok(core) => {
                let tasks = core.db.read(repo::list_tasks).unwrap_or_else(|e| {
                    self.add_issue("database", format!("Couldn't read tasks: {e}"));
                    Vec::new()
                });
                let alerts = core.db.read(repo::active_alerts).unwrap_or_default();
                (tasks, alerts)
            }
            Err(_) => (Vec::new(), Vec::new()),
        };
        Snapshot {
            product_name: self.product_name.clone(),
            version: self.version.clone(),
            tasks,
            alerts,
            settings: self.settings(),
            issues: self.issues.lock().map(|i| i.clone()).unwrap_or_default(),
            db_path: self.db_path.display().to_string(),
            startup_error: self.startup_error.lock().ok().and_then(|e| e.clone()),
            now_utc: self.now(),
        }
    }
}

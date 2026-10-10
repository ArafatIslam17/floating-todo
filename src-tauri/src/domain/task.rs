//! Data types exchanged with the UI. Field names are camelCase on the wire.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Completed,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Completed => "completed",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskStatus::Pending),
            "completed" => Some(TaskStatus::Completed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    pub due_at_utc: Option<i64>,
    pub timezone_id: Option<String>,
    pub lead_minutes: Option<i64>,
    pub status: TaskStatus,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub completed_at_utc: Option<i64>,
    /// When a snoozed reminder will ring again (display only; deadline is unchanged).
    pub snoozed_until_utc: Option<i64>,
}

/// Raw input from the UI before validation.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub title: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub due_at_utc: Option<i64>,
    #[serde(default)]
    pub timezone_id: Option<String>,
    #[serde(default)]
    pub lead_minutes: Option<i64>,
}

/// Input after validation and normalisation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidTaskInput {
    pub title: String,
    pub notes: Option<String>,
    pub due_at_utc: Option<i64>,
    pub timezone_id: Option<String>,
    pub lead_minutes: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReminderType {
    Lead,
    Due,
}

impl ReminderType {
    pub fn as_str(self) -> &'static str {
        match self {
            ReminderType::Lead => "lead",
            ReminderType::Due => "due",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "lead" => Some(ReminderType::Lead),
            "due" => Some(ReminderType::Due),
            _ => None,
        }
    }
}

/// A fired, unacknowledged reminder on a live pending task.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveAlert {
    pub reminder_id: i64,
    pub task_id: String,
    pub title: String,
    pub reminder_type: ReminderType,
    pub due_at_utc: Option<i64>,
    pub fired_at_utc: i64,
}

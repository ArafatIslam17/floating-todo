//! Error type shared by every command. Serialises to `{ code, message }` for the UI.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("{0}")]
    Audio(String),
    #[error("{0}")]
    Notification(String),
    #[error("{0}")]
    Window(String),
    #[error("{0}")]
    Shortcut(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Validation(_) => "validation",
            AppError::NotFound(_) => "not_found",
            AppError::Database(_) => "database",
            AppError::Audio(_) => "audio",
            AppError::Notification(_) => "notification",
            AppError::Window(_) => "window",
            AppError::Shortcut(_) => "shortcut",
            AppError::Internal(_) => "internal",
        }
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<rusqlite_migration::Error> for AppError {
    fn from(e: rusqlite_migration::Error) -> Self {
        AppError::Database(format!("migration failed: {e}"))
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Window(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;

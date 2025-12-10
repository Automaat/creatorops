//! Error types for `CreatorOps` application
//!
//! This module provides a unified error type using thiserror for better error handling
//! and context preservation throughout the application.

use thiserror::Error;

/// Application error type that wraps all possible errors
#[derive(Error, Debug)]
pub enum AppError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// I/O operation failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Network request failed
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Google Drive operation failed
    #[error("Google Drive error: {0}")]
    GoogleDrive(String),

    /// Project not found
    #[error("Project not found: {id}")]
    ProjectNotFound { id: String },

    /// Backup operation cancelled
    #[error("Backup cancelled")]
    BackupCancelled,

    /// Operation cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Lock acquisition failed
    #[error("Failed to acquire lock")]
    LockFailed,

    /// Invalid data or state
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Convert `AppError` to String for Tauri commands
///
/// Tauri commands return Result<T, String>, so we need to convert `AppError` to String
impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::ProjectNotFound {
            id: "test-123".to_owned(),
        };
        assert_eq!(err.to_string(), "Project not found: test-123");
    }

    #[test]
    fn test_error_conversion_to_string() {
        let err = AppError::BackupCancelled;
        let s: String = err.into();
        assert_eq!(s, "Backup cancelled");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        assert!(app_err.to_string().contains("IO error"));
    }
}

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
    ProjectNotFound {
        /// Project identifier
        id: String,
    },

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

    /// External application launch or environment failure
    #[error("{0}")]
    ExternalApp(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Convert `AppError` to String for Tauri commands
///
/// Tauri commands return Result<T, String>, so we need to convert `AppError` to String
impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        err.to_string()
    }
}

/// Errors from backup operations
#[derive(Error, Debug)]
pub enum BackupError {
    /// File I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Checksum mismatch after copy
    #[error("Checksum verification failed")]
    ChecksumMismatch,

    /// Checksum calculation failure
    #[error("Checksum failed: {0}")]
    ChecksumFailed(String),

    /// Source path has no file-name component
    #[error("Invalid source path")]
    InvalidPath,

    /// Path prefix strip failed
    #[error("Path error: {0}")]
    PathError(String),

    /// Recursive file collection failed
    #[error("File collection failed: {0}")]
    CollectFailed(String),

    /// JSON serialization or deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Mutex lock acquisition failed
    #[error("Failed to acquire lock: {0}")]
    LockFailed(String),

    /// Referenced backup job does not exist
    #[error("Backup job not found")]
    JobNotFound,

    /// Tried to cancel a non-pending job
    #[error("Can only cancel pending backups")]
    NotPending,

    /// Tried to remove an in-progress job
    #[error("Cannot remove in-progress backup")]
    InProgress,

    /// Configuration or environment error
    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<BackupError> for String {
    fn from(err: BackupError) -> Self {
        err.to_string()
    }
}

/// Errors from delivery operations
#[derive(Error, Debug)]
pub enum DeliveryError {
    /// File I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// File has no valid name component
    #[error("Invalid file name")]
    InvalidFileName,

    /// Path prefix strip failed
    #[error("Path error: {0}")]
    PathError(String),
}

impl From<DeliveryError> for String {
    fn from(err: DeliveryError) -> Self {
        err.to_string()
    }
}

/// Errors from import (SD card copy) operations
#[derive(Error, Debug)]
pub enum ImportError {
    /// Import was cancelled by the user
    #[error("Import cancelled")]
    Cancelled,

    /// Tokio task join error
    #[error("Task failed: {0}")]
    TaskFailed(String),

    /// Import token not found — session already completed
    #[error("Import not found or already completed")]
    NotFound,

    /// Semaphore permit acquisition failed
    #[error("Semaphore acquire failed: {0}")]
    SemaphoreError(String),

    /// Underlying file copy returned an error
    #[error("File copy failed: {0}")]
    CopyFailed(String),
}

impl From<ImportError> for String {
    fn from(err: ImportError) -> Self {
        err.to_string()
    }
}

/// Errors from Google Drive authentication and API operations
#[derive(Error, Debug)]
pub enum GoogleDriveError {
    /// File I/O error (token files, token directory)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP/network failure
    #[error("Network error: {0}")]
    Network(String),

    /// Non-2xx response from Google API
    #[error("API error: {0}")]
    ApiError(String),

    /// Response parsing or deserialization failure
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// AES-GCM encryption or decryption failure
    #[error("Encryption error: {0}")]
    Crypto(String),

    /// Mutex lock acquisition failed
    #[error("Failed to acquire lock")]
    LockFailed,

    /// No stored token found for the account
    #[error("Token not found")]
    TokenNotFound,

    /// Configuration or environment variable missing
    #[error("Configuration error: {0}")]
    Config(String),

    /// OAuth code exchange timed out
    #[error("Authentication timeout")]
    AuthTimeout,
}

impl From<GoogleDriveError> for String {
    fn from(err: GoogleDriveError) -> Self {
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

    #[test]
    fn test_serde_error_conversion() {
        let serde_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let app_err: AppError = serde_err.into();
        assert!(app_err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_backup_error_display() {
        assert_eq!(
            BackupError::ChecksumMismatch.to_string(),
            "Checksum verification failed"
        );
        assert_eq!(BackupError::InvalidPath.to_string(), "Invalid source path");
        assert_eq!(BackupError::JobNotFound.to_string(), "Backup job not found");
        assert_eq!(
            BackupError::NotPending.to_string(),
            "Can only cancel pending backups"
        );
        assert_eq!(
            BackupError::InProgress.to_string(),
            "Cannot remove in-progress backup"
        );
    }

    #[test]
    fn test_backup_error_to_string_conversion() {
        let s: String = BackupError::JobNotFound.into();
        assert_eq!(s, "Backup job not found");
    }

    #[test]
    fn test_backup_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: BackupError = io_err.into();
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_delivery_error_display() {
        assert_eq!(
            DeliveryError::InvalidFileName.to_string(),
            "Invalid file name"
        );
        assert_eq!(
            DeliveryError::PathError("bad prefix".to_owned()).to_string(),
            "Path error: bad prefix"
        );
    }

    #[test]
    fn test_delivery_error_to_string_conversion() {
        let s: String = DeliveryError::InvalidFileName.into();
        assert_eq!(s, "Invalid file name");
    }

    #[test]
    fn test_import_error_display() {
        assert_eq!(ImportError::Cancelled.to_string(), "Import cancelled");
        assert_eq!(
            ImportError::NotFound.to_string(),
            "Import not found or already completed"
        );
    }

    #[test]
    fn test_import_error_to_string_conversion() {
        let s: String = ImportError::Cancelled.into();
        assert_eq!(s, "Import cancelled");
    }

    #[test]
    fn test_google_drive_error_display() {
        assert_eq!(
            GoogleDriveError::LockFailed.to_string(),
            "Failed to acquire lock"
        );
        assert_eq!(
            GoogleDriveError::TokenNotFound.to_string(),
            "Token not found"
        );
        assert_eq!(
            GoogleDriveError::AuthTimeout.to_string(),
            "Authentication timeout"
        );
        assert_eq!(
            GoogleDriveError::ApiError("403 Forbidden".to_owned()).to_string(),
            "API error: 403 Forbidden"
        );
    }

    #[test]
    fn test_google_drive_error_to_string_conversion() {
        let s: String = GoogleDriveError::TokenNotFound.into();
        assert_eq!(s, "Token not found");
    }
}

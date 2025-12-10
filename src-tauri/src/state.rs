//! Application state management
//!
//! Provides centralized state management for all async operations,
//! replacing lazy_static global mutable state with Tauri-managed state.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::modules::archive::ArchiveJob;
use crate::modules::backup::BackupJob;
use crate::modules::delivery::DeliveryJob;

/// Maximum concurrent file copy operations
const MAX_CONCURRENT_COPIES: usize = 4;

/// Centralized application state managed by Tauri
pub struct AppState {
    /// Backup job queue
    pub backup_queue: Arc<Mutex<HashMap<String, BackupJob>>>,

    /// Delivery job queue
    pub delivery_queue: Arc<Mutex<HashMap<String, DeliveryJob>>>,

    /// Archive job queue
    pub archive_queue: Arc<Mutex<HashMap<String, ArchiveJob>>>,

    /// Import operation cancellation tokens
    pub import_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,

    /// Semaphore for limiting concurrent file copy operations
    pub file_semaphore: Arc<Semaphore>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            backup_queue: Arc::new(Mutex::new(HashMap::new())),
            delivery_queue: Arc::new(Mutex::new(HashMap::new())),
            archive_queue: Arc::new(Mutex::new(HashMap::new())),
            import_tokens: Arc::new(Mutex::new(HashMap::new())),
            file_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_COPIES)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::default();
        assert_eq!(state.file_semaphore.available_permits(), MAX_CONCURRENT_COPIES);
    }
}

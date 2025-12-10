//! Application state management
//!
//! Provides centralized state management for all async operations,
//! replacing `lazy_static` global mutable state with Tauri-managed state.

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

/// Type alias for backup job queue
pub type BackupQueue = Arc<Mutex<HashMap<String, BackupJob>>>;

/// Type alias for delivery job queue
pub type DeliveryQueue = Arc<Mutex<HashMap<String, DeliveryJob>>>;

/// Type alias for archive job queue
pub type ArchiveQueue = Arc<Mutex<HashMap<String, ArchiveJob>>>;

/// Type alias for import cancellation tokens
pub type ImportTokens = Arc<Mutex<HashMap<String, CancellationToken>>>;

/// Centralized application state managed by Tauri
pub struct AppState {
    /// Backup job queue
    pub backup_queue: BackupQueue,

    /// Delivery job queue
    pub delivery_queue: DeliveryQueue,

    /// Archive job queue
    pub archive_queue: ArchiveQueue,

    /// Import operation cancellation tokens
    pub import_tokens: ImportTokens,

    /// Semaphore for limiting concurrent file copy operations
    /// TODO: Integrate with `file_copy.rs` `copy_files` function to use shared semaphore
    #[allow(dead_code)] // Reserved for future use
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
        assert_eq!(
            state.file_semaphore.available_permits(),
            MAX_CONCURRENT_COPIES
        );
    }

    #[tokio::test]
    async fn test_backup_queue_operations() {
        use crate::modules::backup::{BackupJob, BackupStatus};

        let state = AppState::default();
        let job = BackupJob {
            id: "test-job".to_owned(),
            project_id: "proj-1".to_owned(),
            project_name: "Test Project".to_owned(),
            source_path: "/source".to_owned(),
            destination_id: "dest-1".to_owned(),
            destination_name: "Test Drive".to_owned(),
            destination_path: "/dest".to_owned(),
            status: BackupStatus::Pending,
            total_files: 0,
            files_copied: 0,
            files_skipped: 0,
            total_bytes: 0,
            bytes_transferred: 0,
            created_at: "2024-01-01".to_owned(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        {
            let mut queue = state.backup_queue.lock().await;
            queue.insert(job.id.clone(), job.clone());
        }

        {
            let queue = state.backup_queue.lock().await;
            assert!(queue.contains_key(&job.id));
        }
    }

    #[tokio::test]
    async fn test_delivery_queue_operations() {
        use crate::modules::delivery::{DeliveryJob, DeliveryStatus};

        let state = AppState::default();
        let job = DeliveryJob {
            id: "test-delivery".to_owned(),
            project_id: "proj-1".to_owned(),
            project_name: "Test Project".to_owned(),
            selected_files: vec![],
            delivery_path: "/delivery".to_owned(),
            naming_template: None,
            status: DeliveryStatus::Pending,
            total_files: 0,
            files_copied: 0,
            total_bytes: 0,
            bytes_transferred: 0,
            created_at: "2024-01-01".to_owned(),
            started_at: None,
            completed_at: None,
            error_message: None,
            manifest_path: None,
        };

        {
            let mut queue = state.delivery_queue.lock().await;
            queue.insert(job.id.clone(), job.clone());
        }

        {
            let queue = state.delivery_queue.lock().await;
            assert!(queue.contains_key(&job.id));
        }
    }

    #[tokio::test]
    async fn test_archive_queue_operations() {
        use crate::modules::archive::{ArchiveJob, ArchiveStatus};

        let state = AppState::default();
        let job = ArchiveJob {
            id: "test-archive".to_owned(),
            project_id: "proj-1".to_owned(),
            project_name: "Test Project".to_owned(),
            source_path: "/source".to_owned(),
            archive_path: "/archives".to_owned(),
            compress: false,
            compression_format: None,
            status: ArchiveStatus::Pending,
            total_files: 0,
            files_archived: 0,
            total_bytes: 0,
            bytes_transferred: 0,
            created_at: "2024-01-01".to_owned(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        {
            let mut queue = state.archive_queue.lock().await;
            queue.insert(job.id.clone(), job.clone());
        }

        {
            let queue = state.archive_queue.lock().await;
            assert!(queue.contains_key(&job.id));
        }
    }

    #[tokio::test]
    async fn test_import_tokens_operations() {
        let state = AppState::default();
        let token = CancellationToken::new();

        {
            let mut tokens = state.import_tokens.lock().await;
            tokens.insert("import-1".to_owned(), token.clone());
        }

        {
            let tokens = state.import_tokens.lock().await;
            assert!(tokens.contains_key("import-1"));
        }
    }

    #[test]
    fn test_max_concurrent_copies_constant() {
        assert_eq!(MAX_CONCURRENT_COPIES, 4);
    }
}

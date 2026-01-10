/// Progress tracking abstraction (Phase 3 - extract duplicated progress logic, deferred from Phase 2)
use serde::Serialize;
use tauri::{Emitter, Window};

/// Progress reporter trait for operations with progress tracking
#[allow(dead_code)] // Created for future use in Phase 3
pub trait ProgressReporter: Send + Sync {
    fn report(&self, update: ProgressUpdate);
    fn report_error(&self, error: &str);
}

/// Standard progress update structure
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Created for future use in Phase 3
pub struct ProgressUpdate {
    pub job_id: String,
    pub current: usize,
    pub total: usize,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub current_file: Option<String>,
    pub speed_bps: Option<u64>,
    pub eta_seconds: Option<u64>,
}

/// Tauri-based progress reporter (emits events to frontend)
#[allow(dead_code)] // Created for future use in Phase 3
pub struct TauriProgressReporter {
    window: Window,
    event_name: String,
    job_id: String,
}

#[allow(dead_code)] // Created for future use in Phase 3
impl TauriProgressReporter {
    pub const fn new(window: Window, event_name: String, job_id: String) -> Self {
        Self {
            window,
            event_name,
            job_id,
        }
    }
}

impl ProgressReporter for TauriProgressReporter {
    fn report(&self, update: ProgressUpdate) {
        let _ = self.window.emit(&self.event_name, &update);
    }

    fn report_error(&self, error: &str) {
        let _ = self.window.emit(
            &format!("{}-error", self.event_name),
            &serde_json::json!({
                "jobId": self.job_id,
                "error": error
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_update_serialization() {
        let update = ProgressUpdate {
            job_id: "test-123".to_owned(),
            current: 5,
            total: 10,
            bytes_processed: 1024,
            total_bytes: 2048,
            current_file: Some("test.jpg".to_owned()),
            speed_bps: Some(1_000_000),
            eta_seconds: Some(30),
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("jobId")); // Check camelCase
        assert!(json.contains("bytesProcessed"));
        assert!(json.contains("currentFile"));
    }

    #[test]
    fn test_progress_update_with_none_values() {
        let update = ProgressUpdate {
            job_id: "test-456".to_owned(),
            current: 0,
            total: 100,
            bytes_processed: 0,
            total_bytes: 10240,
            current_file: None,
            speed_bps: None,
            eta_seconds: None,
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("jobId"));
        assert!(json.contains("\"currentFile\":null"));
        assert!(json.contains("\"speedBps\":null"));
        assert!(json.contains("\"etaSeconds\":null"));
    }

    #[test]
    fn test_progress_update_clone() {
        let update = ProgressUpdate {
            job_id: "clone-test".to_owned(),
            current: 3,
            total: 10,
            bytes_processed: 512,
            total_bytes: 1024,
            current_file: Some("file.txt".to_owned()),
            speed_bps: Some(500_000),
            eta_seconds: Some(10),
        };

        let cloned = update.clone();
        assert_eq!(update.job_id, cloned.job_id);
        assert_eq!(update.current, cloned.current);
        assert_eq!(update.total, cloned.total);
    }
}

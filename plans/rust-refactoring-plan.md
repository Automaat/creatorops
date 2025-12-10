# CreatorOps Rust Backend Refactoring Plan

**Date:** December 7, 2025
**Status:** Research Complete, Ready for Implementation

## Executive Summary

Comprehensive refactoring plan for CreatorOps Rust/Tauri backend based on 2024-2025 best practices research and codebase analysis. Prioritizes **reliability over reorganization** with phased approach over 4 weeks.

## Current State Assessment

### ✅ Good Patterns Found
- Uses `log` crate (not `eprintln!`)
- SQLite integration with rusqlite
- Async operations with Tokio
- Proper use of Tauri commands
- Signed commits with hooks

### ❌ Critical Issues Identified
- **74 occurrences** of `Result<T, String>` (loses error context)
- **No database transactions** (risk of partial writes)
- **6 modules** using deprecated `lazy_static` pattern
- **Mixed async file I/O patterns** (performance overhead)
- **No centralized error handling**

## Priority Matrix

| Priority | Effort | Impact | Focus Area |
|----------|--------|--------|------------|
| **Critical** | High | Critical | Error handling with thiserror |
| **Critical** | Low | High | Database transactions |
| **High** | Medium | High | Replace lazy_static with state |
| **Medium** | Low | Medium | Optimize async file ops |
| **Medium** | Medium | Medium | Extract progress tracking |
| **Low** | High | Medium | Module reorganization |

## Phase 1: Critical Foundation (Week 1)

### 1. Custom Error Types with thiserror

**Problem:** All modules use `Result<T, String>` losing error context

**Solution:**
```rust
// src-tauri/src/error.rs (new file)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Google Drive error: {0}")]
    GoogleDrive(String),

    #[error("Project not found: {id}")]
    ProjectNotFound { id: String },

    #[error("Backup cancelled")]
    BackupCancelled,

    #[error("Operation cancelled")]
    Cancelled,
}

// For Tauri commands - convert to String
impl From<AppError> for String {
    fn from(err: AppError) -> String {
        err.to_string()
    }
}
```

**Migration Strategy:**
1. Create error.rs with AppError enum
2. Start with db.rs module
3. Migrate one module at a time
4. Update Tauri commands last

### 2. Database Transactions

**Problem:** No transaction support risks partial writes/corrupt state

**Solution:**
```rust
// src-tauri/src/modules/db.rs
impl Database {
    pub fn transaction<F, R>(&self, f: F) -> Result<R, AppError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<R, AppError>,
    {
        let mut conn = self.conn.lock().map_err(|_| AppError::LockFailed)?;
        let tx = conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }
}

// Usage example
pub fn create_project_with_files(project: Project, files: Vec<File>) -> Result<(), AppError> {
    db.transaction(|tx| {
        tx.execute("INSERT INTO projects ...", &project)?;
        for file in files {
            tx.execute("INSERT INTO files ...", &file)?;
        }
        Ok(())
    })
}
```

**Critical Operations to Wrap:**
- Project creation with initial files
- Backup job status updates
- Delivery record creation
- Archive operations

## Phase 2: State Management (Week 2)

### 3. Replace lazy_static with Tauri State

**Problem:** 6 modules use deprecated global mutable state pattern

**Affected Modules:**
- backup.rs (BACKUP_QUEUE, CANCELLATION_TOKENS)
- delivery.rs (DELIVERY_QUEUE, CANCELLATION_TOKENS)
- archive.rs (ARCHIVE_QUEUE)
- file_copy.rs (global semaphore)
- import_history.rs (IMPORT_HISTORY)
- google_drive.rs (CLIENT, TOKEN_CACHE)

**Solution:**
```rust
// src-tauri/src/state.rs
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub backup_queue: Arc<Mutex<HashMap<String, BackupJob>>>,
    pub delivery_queue: Arc<Mutex<HashMap<String, DeliveryJob>>>,
    pub cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub file_semaphore: Arc<Semaphore>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            backup_queue: Arc::new(Mutex::new(HashMap::new())),
            delivery_queue: Arc::new(Mutex::new(HashMap::new())),
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            file_semaphore: Arc::new(Semaphore::new(4)),
        }
    }
}

// In lib.rs
.manage(AppState::default())

// In commands
#[tauri::command]
pub async fn queue_backup(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<BackupJob, AppError> {
    let mut queue = state.backup_queue.lock().await;
    // ... operation
}
```

### 4. Extract Progress Tracking Pattern

**Problem:** Duplicated progress logic across 4 modules

**Solution:**
```rust
// src-tauri/src/progress.rs
use tauri::Window;

pub trait ProgressReporter: Send + Sync {
    fn report(&self, update: ProgressUpdate);
    fn report_error(&self, error: &str);
}

pub struct TauriProgressReporter {
    window: Window,
    event_name: String,
    job_id: String,
}

impl ProgressReporter for TauriProgressReporter {
    fn report(&self, update: ProgressUpdate) {
        let _ = self.window.emit(&self.event_name, &update);
    }

    fn report_error(&self, error: &str) {
        let _ = self.window.emit(&format!("{}-error", self.event_name), json!({
            "job_id": self.job_id,
            "error": error
        }));
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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
```

## Phase 3: Architecture (Week 3)

### 5. Module Reorganization

**Current:** Flat structure with mixed responsibilities

**Proposed Structure:**
```
src-tauri/src/
├── commands/          # Tauri commands only
│   ├── backup.rs     # #[tauri::command] functions
│   ├── project.rs
│   ├── delivery.rs
│   └── settings.rs
├── services/          # Business logic
│   ├── backup_service.rs
│   ├── project_service.rs
│   ├── file_service.rs
│   └── google_drive_service.rs
├── models/           # Data structures
│   ├── backup.rs
│   ├── project.rs
│   ├── delivery.rs
│   └── common.rs
├── utils/
│   ├── file_utils.rs
│   ├── checksum.rs
│   └── media_types.rs
├── db/
│   ├── mod.rs
│   ├── migrations.rs
│   └── queries.rs
├── state.rs          # Application state
├── error.rs          # Error types
├── progress.rs       # Progress tracking
└── lib.rs

```

### 6. Async File Operations Optimization

**Problem:** Unnecessary tokio::fs usage for simple operations

**Solution:**
```rust
// For actual file I/O, use spawn_blocking
pub async fn copy_file(source: &Path, dest: &Path) -> Result<u64, AppError> {
    let source = source.to_path_buf();
    let dest = dest.to_path_buf();

    tokio::task::spawn_blocking(move || {
        std::fs::copy(&source, &dest)
    })
    .await
    .map_err(|e| AppError::Io(e.into()))?
}

// Only use tokio::fs for:
// - Operations that need cancellation
// - Operations with progress tracking
// - Long-running operations that shouldn't block
```

## Phase 4: Polish (Week 4)

### 7. Consolidate File Type Detection

```rust
// src-tauri/src/utils/media_types.rs
#[derive(Debug, Clone, PartialEq)]
pub enum MediaType {
    Photo,
    Video,
    Raw,
    Sidecar,
    Unknown,
}

lazy_static! {
    static ref MEDIA_EXTENSIONS: HashMap<&'static str, MediaType> = {
        let mut m = HashMap::new();
        // Photos
        m.insert("jpg", MediaType::Photo);
        m.insert("jpeg", MediaType::Photo);
        m.insert("png", MediaType::Photo);
        // RAW
        m.insert("arw", MediaType::Raw);
        m.insert("cr2", MediaType::Raw);
        m.insert("nef", MediaType::Raw);
        // Video
        m.insert("mp4", MediaType::Video);
        m.insert("mov", MediaType::Video);
        // Sidecar
        m.insert("xmp", MediaType::Sidecar);
        m
    };
}

pub fn detect_media_type(path: &Path) -> MediaType {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| MEDIA_EXTENSIONS.get(ext.to_lowercase().as_str()))
        .cloned()
        .unwrap_or(MediaType::Unknown)
}
```

### 8. Performance Optimizations

- Replace `Mutex` with `RwLock` for read-heavy operations
- Use `Arc<str>` instead of `String` for frequently cloned immutable strings
- Consider `parking_lot::Mutex` for better performance
- Implement connection pooling for database

## Testing Strategy

### Before Each Refactor
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_behavior() {
        // Capture current behavior
    }

    #[test]
    fn test_edge_cases() {
        // Document edge cases
    }
}
```

### After Each Refactor
- Run existing tests
- Add new tests for error scenarios
- Benchmark performance changes
- Test cancellation scenarios

## Migration Checklist

### Week 1
- [ ] Create error.rs with AppError types
- [ ] Migrate db.rs to use AppError
- [ ] Add transaction support to db.rs
- [ ] Wrap critical operations in transactions
- [ ] Test rollback scenarios

### Week 2
- [ ] Create state.rs with AppState
- [ ] Remove lazy_static from backup.rs
- [ ] Remove lazy_static from delivery.rs
- [ ] Create progress.rs with ProgressReporter
- [ ] Migrate progress tracking in all modules

### Week 3
- [ ] Create new directory structure
- [ ] Split commands from services
- [ ] Move models to dedicated directory
- [ ] Optimize async file operations
- [ ] Benchmark performance improvements

### Week 4
- [ ] Consolidate file type detection
- [ ] Remove format!() error messages
- [ ] Add builder patterns where appropriate
- [ ] Performance optimizations
- [ ] Documentation updates

## Success Metrics

- **Zero** `Result<T, String>` occurrences
- **All** multi-step operations use transactions
- **No** global mutable state
- **50%** reduction in duplicated code
- **All** tests passing
- **Improved** error messages in logs

## Risk Mitigation

1. **Incremental Migration:** One module at a time
2. **Feature Flags:** Can toggle between old/new implementations
3. **Comprehensive Testing:** Before and after each change
4. **Rollback Plan:** Git tags at each successful phase

## Unresolved Questions

1. **Error Library Choice:** thiserror vs anyhow?
   - **Recommendation:** thiserror for libraries, anyhow for applications
   - Since this is application code: **use thiserror for typed errors**

2. **Module Structure:** Flat vs nested?
   - **Recommendation:** Start flat, nest only when > 10 files per directory

3. **Priority:** Reliability vs Performance?
   - **Decision:** **Reliability first, always**

## References

- [Patterns for Defensive Programming in Rust](https://corrode.dev/blog/defensive-programming/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Error Handling for Large Rust Projects](https://greptime.com/blogs/2024-05-07-error-rust)
- [The State of Async Rust: Runtimes](https://corrode.dev/blog/async/)
- [Tauri Architecture Best Practices](https://v2.tauri.app/concept/architecture/)

## Implementation Start

Begin with **Phase 1: Custom Error Types** as it provides foundation for all other improvements.
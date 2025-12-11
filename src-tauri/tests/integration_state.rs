//! Integration tests for state-managed Tauri commands
//!
//! These tests verify that the _impl functions work correctly with `AppState`

use creatorops_lib::{
    cancel_backup_impl, create_archive_impl, create_delivery_impl, get_archive_queue_impl,
    get_backup_queue_impl, get_delivery_queue_impl, queue_backup_impl, remove_archive_job_impl,
    remove_backup_job_impl, remove_delivery_job_impl, state::AppState,
};

#[cfg(test)]
mod backup_integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_queue_backup_with_state() {
        let state = AppState::default();
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "test data").unwrap();

        let result = queue_backup_impl(
            &state.backup_queue,
            "proj-123".to_owned(),
            "Test Project".to_owned(),
            source.to_string_lossy().to_string(),
            "dest-456".to_owned(),
            "Test Drive".to_owned(),
            "/backup".to_owned(),
        )
        .await;

        assert!(result.is_ok());
        let job = result.unwrap();
        assert_eq!(job.project_id, "proj-123");

        // Cleanup
        let _ = remove_backup_job_impl(&state.backup_queue, job.id).await;
    }

    #[tokio::test]
    async fn test_get_backup_queue_with_state() {
        let state = AppState::default();
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup_impl(
            &state.backup_queue,
            "proj-789".to_owned(),
            "Queue Test".to_owned(),
            source.to_string_lossy().to_string(),
            "dest-123".to_owned(),
            "Test Drive".to_owned(),
            "/backup".to_owned(),
        )
        .await
        .unwrap();

        let queue = get_backup_queue_impl(&state.backup_queue).await.unwrap();
        assert!(queue.iter().any(|j| j.id == job.id));

        // Cleanup
        let _ = remove_backup_job_impl(&state.backup_queue, job.id).await;
    }

    #[tokio::test]
    async fn test_cancel_backup_with_state() {
        let state = AppState::default();
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup_impl(
            &state.backup_queue,
            "proj-cancel".to_owned(),
            "Cancel Test".to_owned(),
            source.to_string_lossy().to_string(),
            "dest-456".to_owned(),
            "Test Drive".to_owned(),
            "/backup".to_owned(),
        )
        .await
        .unwrap();

        let result = cancel_backup_impl(&state.backup_queue, job.id.clone()).await;
        assert!(result.is_ok());

        // Cleanup
        let _ = remove_backup_job_impl(&state.backup_queue, job.id).await;
    }

    #[tokio::test]
    async fn test_remove_backup_job_with_state() {
        let state = AppState::default();
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let job = queue_backup_impl(
            &state.backup_queue,
            "proj-remove".to_owned(),
            "Remove Test".to_owned(),
            source.to_string_lossy().to_string(),
            "dest-789".to_owned(),
            "Test Drive".to_owned(),
            "/backup".to_owned(),
        )
        .await
        .unwrap();

        let result = remove_backup_job_impl(&state.backup_queue, job.id.clone()).await;
        assert!(result.is_ok());

        let queue = get_backup_queue_impl(&state.backup_queue).await.unwrap();
        assert!(!queue.iter().any(|j| j.id == job.id));
    }
}

#[cfg(test)]
mod archive_integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_archive_with_state() {
        let state = AppState::default();
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "test data").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let result = create_archive_impl(
            &state.archive_queue,
            "proj-123".to_owned(),
            "Archive Test".to_owned(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await;

        assert!(result.is_ok());
        let job = result.unwrap();
        assert_eq!(job.project_id, "proj-123");

        // Cleanup
        let _ = remove_archive_job_impl(&state.archive_queue, job.id).await;
    }

    #[tokio::test]
    async fn test_get_archive_queue_with_state() {
        let state = AppState::default();
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let job = create_archive_impl(
            &state.archive_queue,
            "proj-456".to_owned(),
            "Queue Test".to_owned(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await
        .unwrap();

        let queue = get_archive_queue_impl(&state.archive_queue).await.unwrap();
        assert!(queue.iter().any(|j| j.id == job.id));

        // Cleanup
        let _ = remove_archive_job_impl(&state.archive_queue, job.id).await;
    }

    #[tokio::test]
    async fn test_remove_archive_job_with_state() {
        let state = AppState::default();
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("project");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file.txt"), "data").unwrap();

        let archive_location = temp_dir.path().join("archives");
        std::fs::create_dir(&archive_location).unwrap();

        let job = create_archive_impl(
            &state.archive_queue,
            "proj-789".to_owned(),
            "Remove Test".to_owned(),
            source.to_string_lossy().to_string(),
            archive_location.to_string_lossy().to_string(),
            false,
            None,
        )
        .await
        .unwrap();

        let result = remove_archive_job_impl(&state.archive_queue, job.id.clone()).await;
        assert!(result.is_ok());

        let queue = get_archive_queue_impl(&state.archive_queue).await.unwrap();
        assert!(!queue.iter().any(|j| j.id == job.id));
    }
}

#[cfg(test)]
mod delivery_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_delivery_with_state() {
        let state = AppState::default();

        let result = create_delivery_impl(
            &state.delivery_queue,
            "proj-123".to_owned(),
            "Test Project".to_owned(),
            vec!["file1.jpg".to_owned()],
            "/delivery".to_owned(),
            None,
        )
        .await;

        assert!(result.is_ok());
        let job = result.unwrap();
        assert_eq!(job.project_id, "proj-123");

        // Cleanup
        let _ = remove_delivery_job_impl(&state.delivery_queue, job.id).await;
    }

    #[tokio::test]
    async fn test_get_delivery_queue_with_state() {
        let state = AppState::default();

        let job = create_delivery_impl(
            &state.delivery_queue,
            "proj-456".to_owned(),
            "Queue Test".to_owned(),
            vec!["file1.jpg".to_owned()],
            "/delivery".to_owned(),
            None,
        )
        .await
        .unwrap();

        let queue = get_delivery_queue_impl(&state.delivery_queue)
            .await
            .unwrap();
        assert!(queue.iter().any(|j| j.id == job.id));

        // Cleanup
        let _ = remove_delivery_job_impl(&state.delivery_queue, job.id).await;
    }

    #[tokio::test]
    async fn test_remove_delivery_job_with_state() {
        let state = AppState::default();

        let job = create_delivery_impl(
            &state.delivery_queue,
            "proj-789".to_owned(),
            "Remove Test".to_owned(),
            vec!["file1.jpg".to_owned()],
            "/delivery".to_owned(),
            None,
        )
        .await
        .unwrap();

        let result = remove_delivery_job_impl(&state.delivery_queue, job.id.clone()).await;
        assert!(result.is_ok());

        let queue = get_delivery_queue_impl(&state.delivery_queue)
            .await
            .unwrap();
        assert!(!queue.iter().any(|j| j.id == job.id));
    }
}

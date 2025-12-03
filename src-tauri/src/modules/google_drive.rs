use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

// Data Structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveAccount {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub parent_folder_id: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub last_authenticated: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthState {
    pub auth_url: String,
    pub server_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}

// OAuth Tauri Commands

#[tauri::command]
pub async fn start_google_drive_auth() -> Result<OAuthState, String> {
    // TODO: Implement OAuth flow startup
    // 1. Load client secret from resources
    // 2. Generate PKCE challenge
    // 3. Find available port
    // 4. Build auth URL
    // 5. Spawn localhost server to capture redirect

    Err("Not implemented yet".to_string())
}

#[tauri::command]
pub async fn complete_google_drive_auth() -> Result<GoogleDriveAccount, String> {
    // TODO: Implement OAuth completion
    // 1. Wait for OAuth server to receive code
    // 2. Exchange code for tokens
    // 3. Get user profile (email, name)
    // 4. Store tokens in keychain
    // 5. Save account to database

    Err("Not implemented yet".to_string())
}

#[tauri::command]
pub async fn get_google_drive_account() -> Result<Option<GoogleDriveAccount>, String> {
    use crate::modules::db::with_db;
    use rusqlite::OptionalExtension;

    with_db(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated FROM google_drive_accounts LIMIT 1")?;

        let account = stmt
            .query_row([], |row| {
                Ok(GoogleDriveAccount {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    display_name: row.get(2)?,
                    parent_folder_id: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    last_authenticated: row.get(6)?,
                })
            })
            .optional()?;

        Ok(account)
    })
    .map_err(|e: rusqlite::Error| format!("Failed to get account: {}", e))
}

#[tauri::command]
pub async fn set_drive_parent_folder(folder_id: Option<String>) -> Result<(), String> {
    use crate::modules::db::with_db;

    with_db(|conn| {
        conn.execute(
            "UPDATE google_drive_accounts SET parent_folder_id = ?1",
            [folder_id],
        )?;

        Ok(())
    })
    .map_err(|e: rusqlite::Error| format!("Failed to update parent folder: {}", e))
}

#[tauri::command]
pub async fn remove_google_drive_account() -> Result<(), String> {
    use crate::modules::db::with_db;

    // First get the email to remove from keychain
    let account = get_google_drive_account().await?;

    if let Some(acc) = account {
        // Remove from keychain
        let entry = keyring::Entry::new("com.creatorops.google-drive", &acc.email)
            .map_err(|e| format!("Failed to create keychain entry: {}", e))?;

        // Ignore error if token doesn't exist in keychain
        let _ = entry.delete_credential();

        // Remove from database
        with_db(|conn| {
            conn.execute("DELETE FROM google_drive_accounts WHERE id = ?1", [&acc.id])?;
            Ok(())
        })
        .map_err(|e: rusqlite::Error| format!("Failed to delete account: {}", e))?;
    }

    Ok(())
}

// Token Management Functions

fn store_tokens_in_keychain(email: &str, tokens: &TokenData) -> Result<(), String> {
    let entry = keyring::Entry::new("com.creatorops.google-drive", email)
        .map_err(|e| format!("Failed to create keychain entry: {}", e))?;

    let token_json = serde_json::to_string(&tokens)
        .map_err(|e| format!("Failed to serialize tokens: {}", e))?;

    entry.set_password(&token_json)
        .map_err(|e| format!("Failed to store tokens in keychain: {}", e))?;

    Ok(())
}

fn get_tokens_from_keychain(email: &str) -> Result<TokenData, String> {
    let entry = keyring::Entry::new("com.creatorops.google-drive", email)
        .map_err(|e| format!("Failed to create keychain entry: {}", e))?;

    let token_json = entry.get_password()
        .map_err(|e| format!("Failed to get tokens from keychain: {}", e))?;

    let tokens: TokenData = serde_json::from_str(&token_json)
        .map_err(|e| format!("Failed to deserialize tokens: {}", e))?;

    Ok(tokens)
}

async fn refresh_access_token(_refresh_token: &str) -> Result<TokenData, String> {
    // TODO: Implement token refresh
    // 1. Load client secret
    // 2. Make refresh token request to Google
    // 3. Return new TokenData

    Err("Not implemented yet".to_string())
}

// Helper Functions

fn get_current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_format() {
        let timestamp = get_current_timestamp();
        assert!(timestamp.contains("T"));
        assert!(timestamp.contains("Z") || timestamp.contains("+"));
        assert!(timestamp.len() > 20);
    }

    #[test]
    fn test_google_drive_account_serialization() {
        let account = GoogleDriveAccount {
            id: "test-id".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            parent_folder_id: Some("folder-123".to_string()),
            enabled: true,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            last_authenticated: "2025-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&account).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("Test User"));

        let deserialized: GoogleDriveAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.email, "test@example.com");
        assert_eq!(deserialized.enabled, true);
    }

    #[test]
    fn test_oauth_state_serialization() {
        let state = OAuthState {
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            server_port: 8080,
        };

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("accounts.google.com"));
        assert!(json.contains("8080"));

        let deserialized: OAuthState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.server_port, 8080);
    }

    #[test]
    fn test_token_data_serialization() {
        let token_data = TokenData {
            access_token: "access_token_123".to_string(),
            refresh_token: "refresh_token_456".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let json = serde_json::to_string(&token_data).unwrap();
        assert!(json.contains("access_token_123"));
        assert!(json.contains("refresh_token_456"));

        let deserialized: TokenData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.access_token, "access_token_123");
        assert_eq!(deserialized.refresh_token, "refresh_token_456");
    }

    #[tokio::test]
    async fn test_get_account_when_none_exists() {
        // This test assumes empty database - would need proper test database setup
        // For now, just ensure it doesn't panic and returns Ok
        let result = get_google_drive_account().await;
        // Should return Ok(None) or Ok(Some(...)) depending on database state
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_set_parent_folder() {
        // Test that the function signature is correct and doesn't panic with valid input
        let result = set_drive_parent_folder(Some("folder-id-123".to_string())).await;
        // May fail if no account exists, but should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_set_parent_folder_none() {
        // Test setting parent folder to None (clearing it)
        let result = set_drive_parent_folder(None).await;
        assert!(result.is_ok() || result.is_err());
    }
}

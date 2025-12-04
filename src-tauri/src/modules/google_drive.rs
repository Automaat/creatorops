#![allow(clippy::unreachable)] // False positive: Clippy incorrectly flags Result returns

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::modules::db::Database;

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
#[serde(rename_all = "camelCase")]
pub struct OAuthState {
    pub auth_url: String,
    pub server_port: u16,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    Err("Not implemented yet".to_owned())
}

#[tauri::command]
pub async fn complete_google_drive_auth() -> Result<GoogleDriveAccount, String> {
    // TODO: Implement OAuth completion
    // 1. Wait for OAuth server to receive code
    // 2. Exchange code for tokens
    // 3. Get user profile (email, name)
    // 4. Store tokens in keychain
    // 5. Save account to database

    Err("Not implemented yet".to_owned())
}

#[tauri::command]
pub async fn get_google_drive_account(
    db: tauri::State<'_, Database>,
) -> Result<Option<GoogleDriveAccount>, String> {
    use rusqlite::OptionalExtension;

    db.execute(|conn| {
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
    .map_err(|e: rusqlite::Error| format!("Failed to get account: {e}"))
}

#[tauri::command]
pub async fn set_drive_parent_folder(
    db: tauri::State<'_, Database>,
    folder_id: Option<String>,
) -> Result<(), String> {
    // Get the current account to ensure we only update one account
    let account = get_google_drive_account(db.clone()).await?;

    account.map_or_else(
        || Err("No Google Drive account found to update parent folder".to_owned()),
        |acc| {
            db.execute(|conn| {
                conn.execute(
                    "UPDATE google_drive_accounts SET parent_folder_id = ?1 WHERE id = ?2",
                    [&folder_id, &Some(acc.id)],
                )?;
                Ok(())
            })
            .map_err(|e: rusqlite::Error| format!("Failed to update parent folder: {e}"))
        },
    )
}

#[tauri::command]
pub async fn remove_google_drive_account(db: tauri::State<'_, Database>) -> Result<(), String> {
    // First get the email to remove from keychain
    let account = get_google_drive_account(db.clone()).await?;

    if let Some(acc) = account {
        // Remove from keychain
        let entry = keyring::Entry::new("com.creatorops.google-drive", &acc.email)
            .map_err(|e| format!("Failed to create keychain entry: {e}"))?;

        // Ignore error if token doesn't exist in keychain
        let _ = entry.delete_credential();

        // Remove from database
        db.execute(|conn| {
            conn.execute("DELETE FROM google_drive_accounts WHERE id = ?1", [&acc.id])?;
            Ok(())
        })
        .map_err(|e: rusqlite::Error| format!("Failed to delete account: {e}"))?;
    }

    Ok(())
}

// Token Management Functions

#[allow(dead_code)]
fn store_tokens_in_keychain(email: &str, tokens: &TokenData) -> Result<(), String> {
    let entry = keyring::Entry::new("com.creatorops.google-drive", email)
        .map_err(|e| format!("Failed to create keychain entry: {e}"))?;

    let token_json =
        serde_json::to_string(&tokens).map_err(|e| format!("Failed to serialize tokens: {e}"))?;

    entry
        .set_password(&token_json)
        .map_err(|e| format!("Failed to store tokens in keychain: {e}"))?;

    Ok(())
}

#[allow(dead_code)]
fn get_tokens_from_keychain(email: &str) -> Result<TokenData, String> {
    let entry = keyring::Entry::new("com.creatorops.google-drive", email)
        .map_err(|e| format!("Failed to create keychain entry: {e}"))?;

    let token_json = entry
        .get_password()
        .map_err(|e| format!("Failed to get tokens from keychain: {e}"))?;

    let tokens: TokenData = serde_json::from_str(&token_json)
        .map_err(|e| format!("Failed to deserialize tokens: {e}"))?;

    Ok(tokens)
}

#[allow(dead_code)]
fn refresh_access_token(_refresh_token: &str) -> Result<TokenData, String> {
    // TODO: Implement token refresh (will be async when implemented)
    // 1. Load client secret
    // 2. Make refresh token request to Google
    // 3. Return new TokenData

    Err("Not implemented yet".to_owned())
}

// Helper Functions

#[allow(dead_code)]
fn get_current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_timestamp_format() {
        let timestamp = get_current_timestamp();
        assert!(timestamp.contains('T'));
        assert!(timestamp.contains('Z') || timestamp.contains('+'));
        assert!(timestamp.len() > 20);
    }

    #[test]
    fn test_google_drive_account_serialization() {
        let account = GoogleDriveAccount {
            id: "test-id".to_owned(),
            email: "test@example.com".to_owned(),
            display_name: "Test User".to_owned(),
            parent_folder_id: Some("folder-123".to_owned()),
            enabled: true,
            created_at: "2025-01-01T00:00:00Z".to_owned(),
            last_authenticated: "2025-01-01T00:00:00Z".to_owned(),
        };

        let json = serde_json::to_string(&account).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("Test User"));

        let deserialized: GoogleDriveAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.email, "test@example.com");
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_oauth_state_serialization() {
        let state = OAuthState {
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_owned(),
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
            access_token: "access_token_123".to_owned(),
            refresh_token: "refresh_token_456".to_owned(),
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
    async fn test_start_google_drive_auth_not_implemented() {
        let result = start_google_drive_auth().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not implemented yet");
    }

    #[tokio::test]
    async fn test_complete_google_drive_auth_not_implemented() {
        let result = complete_google_drive_auth().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not implemented yet");
    }

    #[test]
    fn test_refresh_access_token_not_implemented() {
        let result = refresh_access_token("test_token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not implemented yet");
    }

    // Database-dependent tests
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, Database) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new_with_path(&db_path).unwrap();
        (temp_dir, db)
    }

    #[test]
    fn test_db_get_google_drive_account_none() {
        use rusqlite::OptionalExtension;

        let (_temp_dir, db) = setup_test_db();
        let result = db
            .execute(|conn| {
                let mut stmt = conn.prepare("SELECT id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated FROM google_drive_accounts LIMIT 1")?;
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
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_db_get_google_drive_account_exists() {
        use rusqlite::OptionalExtension;

        let (_temp_dir, db) = setup_test_db();

        // Insert account
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO google_drive_accounts (id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "test-id",
                    "test@example.com",
                    "Test User",
                    Some("folder-123"),
                    1,
                    "2025-01-01T00:00:00Z",
                    "2025-01-01T00:00:00Z",
                ],
            )?;
            Ok(())
        })
        .unwrap();

        let result = db
            .execute(|conn| {
                let mut stmt = conn.prepare("SELECT id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated FROM google_drive_accounts LIMIT 1")?;
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
            .unwrap();

        assert!(result.is_some());
        let account = result.unwrap();
        assert_eq!(account.id, "test-id");
        assert_eq!(account.email, "test@example.com");
        assert_eq!(account.display_name, "Test User");
        assert_eq!(account.parent_folder_id, Some("folder-123".to_owned()));
        assert!(account.enabled);
    }

    #[test]
    fn test_db_get_google_drive_account_disabled() {
        use rusqlite::OptionalExtension;

        let (_temp_dir, db) = setup_test_db();

        // Insert disabled account
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO google_drive_accounts (id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "disabled-id",
                    "disabled@example.com",
                    "Disabled User",
                    None::<String>,
                    0,
                    "2025-01-01T00:00:00Z",
                    "2025-01-01T00:00:00Z",
                ],
            )?;
            Ok(())
        })
        .unwrap();

        let result = db
            .execute(|conn| {
                let mut stmt = conn.prepare("SELECT id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated FROM google_drive_accounts LIMIT 1")?;
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
            .unwrap();

        assert!(result.is_some());
        let account = result.unwrap();
        assert!(!account.enabled);
        assert_eq!(account.parent_folder_id, None);
    }

    #[test]
    fn test_db_set_drive_parent_folder_success() {
        use rusqlite::OptionalExtension;

        let (_temp_dir, db) = setup_test_db();

        // Insert account
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO google_drive_accounts (id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "account-1",
                    "user@example.com",
                    "User Name",
                    None::<String>,
                    1,
                    "2025-01-01T00:00:00Z",
                    "2025-01-01T00:00:00Z",
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Set parent folder
        db.execute(|conn| {
            conn.execute(
                "UPDATE google_drive_accounts SET parent_folder_id = ?1 WHERE id = ?2",
                [&Some("new-folder-id"), &Some("account-1")],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify it was set
        let account: Option<GoogleDriveAccount> = db
            .execute(|conn| {
                let mut stmt = conn.prepare("SELECT id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated FROM google_drive_accounts WHERE id = ?1")?;
                stmt.query_row(["account-1"], |row| {
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
                .optional()
            })
            .unwrap();

        assert!(account.is_some());
        assert_eq!(
            account.unwrap().parent_folder_id,
            Some("new-folder-id".to_owned())
        );
    }

    #[test]
    fn test_db_set_drive_parent_folder_clear() {
        use rusqlite::OptionalExtension;

        let (_temp_dir, db) = setup_test_db();

        // Insert account with folder
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO google_drive_accounts (id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "account-2",
                    "user2@example.com",
                    "User Two",
                    Some("existing-folder"),
                    1,
                    "2025-01-01T00:00:00Z",
                    "2025-01-01T00:00:00Z",
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Clear parent folder
        db.execute(|conn| {
            conn.execute(
                "UPDATE google_drive_accounts SET parent_folder_id = ?1 WHERE id = ?2",
                rusqlite::params![None::<String>, "account-2"],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify it was cleared
        let account: Option<GoogleDriveAccount> = db
            .execute(|conn| {
                let mut stmt = conn.prepare("SELECT id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated FROM google_drive_accounts WHERE id = ?1")?;
                stmt.query_row(["account-2"], |row| {
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
                .optional()
            })
            .unwrap();

        assert!(account.is_some());
        assert_eq!(account.unwrap().parent_folder_id, None);
    }

    #[test]
    fn test_db_remove_google_drive_account_success() {
        let (_temp_dir, db) = setup_test_db();

        // Insert account
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO google_drive_accounts (id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "remove-id",
                    "remove@example.com",
                    "Remove User",
                    None::<String>,
                    1,
                    "2025-01-01T00:00:00Z",
                    "2025-01-01T00:00:00Z",
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify account exists
        let count: i32 = db
            .execute(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM google_drive_accounts WHERE id = ?1",
                    ["remove-id"],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert_eq!(count, 1);

        // Remove account
        db.execute(|conn| {
            conn.execute(
                "DELETE FROM google_drive_accounts WHERE id = ?1",
                ["remove-id"],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify account is gone
        let count: i32 = db
            .execute(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM google_drive_accounts WHERE id = ?1",
                    ["remove-id"],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_store_tokens_serialization() {
        // Test token serialization without keychain
        let token_data = TokenData {
            access_token: "test_access_token".to_owned(),
            refresh_token: "test_refresh_token".to_owned(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let json = serde_json::to_string(&token_data).unwrap();
        assert!(json.contains("test_access_token"));
        assert!(json.contains("test_refresh_token"));

        let deserialized: TokenData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.access_token, "test_access_token");
        assert_eq!(deserialized.refresh_token, "test_refresh_token");
    }

    #[test]
    fn test_get_tokens_from_keychain_not_found() {
        let nonexistent_email = format!("nonexistent-{}@example.com", Uuid::new_v4());
        let result = get_tokens_from_keychain(&nonexistent_email);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Failed to get tokens from keychain"));
    }

    #[test]
    fn test_oauth_state_camel_case_serialization() {
        let state = OAuthState {
            auth_url: "https://example.com/auth".to_owned(),
            server_port: 3000,
        };

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("authUrl"));
        assert!(json.contains("serverPort"));
        assert!(!json.contains("auth_url"));
        assert!(!json.contains("server_port"));
    }

    #[test]
    fn test_token_data_camel_case_serialization() {
        let token_data = TokenData {
            access_token: "access".to_owned(),
            refresh_token: "refresh".to_owned(),
            expires_at: Utc::now(),
        };

        let json = serde_json::to_string(&token_data).unwrap();
        assert!(json.contains("accessToken"));
        assert!(json.contains("refreshToken"));
        assert!(json.contains("expiresAt"));
        assert!(!json.contains("access_token"));
        assert!(!json.contains("refresh_token"));
        assert!(!json.contains("expires_at"));
    }

    #[test]
    fn test_google_drive_account_with_none_values() {
        let account = GoogleDriveAccount {
            id: "test".to_owned(),
            email: "test@example.com".to_owned(),
            display_name: "Test".to_owned(),
            parent_folder_id: None,
            enabled: false,
            created_at: get_current_timestamp(),
            last_authenticated: get_current_timestamp(),
        };

        let json = serde_json::to_string(&account).unwrap();
        let deserialized: GoogleDriveAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.parent_folder_id, None);
        assert!(!deserialized.enabled);
    }

    #[test]
    fn test_db_get_google_drive_account_multiple_rows() {
        use rusqlite::OptionalExtension;

        let (_temp_dir, db) = setup_test_db();

        // Insert multiple accounts (edge case)
        db.execute(|conn| {
            conn.execute(
                "INSERT INTO google_drive_accounts (id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "first-id",
                    "first@example.com",
                    "First User",
                    None::<String>,
                    1,
                    "2025-01-01T00:00:00Z",
                    "2025-01-01T00:00:00Z",
                ],
            )?;
            conn.execute(
                "INSERT INTO google_drive_accounts (id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "second-id",
                    "second@example.com",
                    "Second User",
                    None::<String>,
                    1,
                    "2025-01-02T00:00:00Z",
                    "2025-01-02T00:00:00Z",
                ],
            )?;
            Ok(())
        })
        .unwrap();

        // Should return first account only (LIMIT 1)
        let result = db
            .execute(|conn| {
                let mut stmt = conn.prepare("SELECT id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated FROM google_drive_accounts LIMIT 1")?;
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
            .unwrap();
        assert!(result.is_some());
    }
}

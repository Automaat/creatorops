#![allow(clippy::unreachable)] // False positive: Clippy incorrectly flags Result returns

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::oneshot;

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

#[derive(Debug, Clone)]
struct PkceData {
    verifier: String,
    challenge: String,
}

type CodeSender = Arc<Mutex<Option<oneshot::Sender<String>>>>;
type CodeReceiver = Arc<Mutex<Option<oneshot::Receiver<String>>>>;

#[derive(Debug, Clone)]
struct OAuthSession {
    pkce: PkceData,
    state: String,
    port: u16,
    code_sender: CodeSender,
}

lazy_static::lazy_static! {
    static ref OAUTH_SESSION: Arc<Mutex<Option<OAuthSession>>> = Arc::new(Mutex::new(None));
    static ref OAUTH_CODE_RECEIVER: CodeReceiver = Arc::new(Mutex::new(None));
}

const OAUTH_TIMEOUT_SECS: u64 = 300;

// Drop guard to ensure OAuth session cleanup
struct SessionCleanup;
impl Drop for SessionCleanup {
    fn drop(&mut self) {
        let _ = OAUTH_SESSION.lock().map(|mut guard| *guard = None);
    }
}

// OAuth Helper Functions

fn generate_random_alphanumeric(length: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            match idx {
                0..=25 => (b'A' + idx) as char,
                26..=51 => (b'a' + (idx - 26)) as char,
                _ => (b'0' + (idx - 52)) as char,
            }
        })
        .collect()
}

fn generate_pkce() -> PkceData {
    // Generate random verifier (43-128 characters)
    let verifier = generate_random_alphanumeric(128);

    // Generate challenge: BASE64URL(SHA256(verifier))
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(hash);

    PkceData {
        verifier,
        challenge,
    }
}

fn generate_state() -> String {
    generate_random_alphanumeric(32)
}

async fn handle_oauth_redirect(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let uri = req.uri();
    let query = uri.query().unwrap_or("");

    // Parse query parameters
    let params: std::collections::HashMap<String, String> = query
        .split('&')
        .filter_map(|param| {
            let mut parts = param.splitn(2, '=');
            Some((parts.next()?.to_owned(), parts.next()?.to_owned()))
        })
        .collect();

    // Verify state parameter
    let session_data = {
        let session_guard = OAUTH_SESSION
            .lock()
            .map_err(|e| format!("Failed to lock OAuth session: {e}"))?;
        session_guard.clone()
    };

    if let Some(session) = session_data.as_ref() {
        if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
            if state == &session.state {
                // Send code through channel
                if let Ok(mut sender_guard) = session.code_sender.lock() {
                    if let Some(sender) = sender_guard.take() {
                        let _ = sender.send(code.clone());
                    }
                }

                let response_body = r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="UTF-8">
                        <title>CreatorOps - Authorization Successful</title>
                    </head>
                    <body style="font-family: system-ui; text-align: center; padding: 50px;">
                        <h1>✅ Authorization Successful</h1>
                        <p>You can close this window and return to CreatorOps.</p>
                    </body>
                    </html>
                "#;

                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/html")
                    .body(Full::new(Bytes::from(response_body)))?);
            }
        }
    }

    // Error case
    let error_body = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>CreatorOps - Authorization Failed</title>
        </head>
        <body style="font-family: system-ui; text-align: center; padding: 50px;">
            <h1>❌ Authorization Failed</h1>
            <p>Please try again in CreatorOps.</p>
        </body>
        </html>
    "#;

    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(error_body)))?)
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    email: String,
    name: String,
}

async fn exchange_code_for_tokens(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<TokenResponse, String> {
    let client = reqwest::Client::new();

    let params = [
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token exchange request failed: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_owned());
        return Err(format!("Token exchange failed: {error_text}"));
    }

    response
        .json::<TokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))
}

async fn get_user_info(access_token: &str) -> Result<UserInfo, String> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("User info request failed: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_owned());
        return Err(format!("User info request failed: {error_text}"));
    }

    response
        .json::<UserInfo>()
        .await
        .map_err(|e| format!("Failed to parse user info: {e}"))
}

// OAuth Tauri Commands

#[tauri::command]
pub async fn start_google_drive_auth() -> Result<OAuthState, String> {
    // 1. Generate PKCE challenge
    let pkce = generate_pkce();
    let state = generate_state();

    // 2. Use fixed port for OAuth redirect
    let port = 8080;

    // 3. Create channel for auth code
    let (tx, rx) = oneshot::channel::<String>();

    // 4. Store session
    {
        let mut session_guard = OAUTH_SESSION
            .lock()
            .map_err(|_| "Failed to lock OAuth session".to_owned())?;
        *session_guard = Some(OAuthSession {
            pkce: pkce.clone(),
            state: state.clone(),
            port,
            code_sender: Arc::new(Mutex::new(Some(tx))),
        });
    }

    // 5. Spawn HTTP server
    let addr = format!("127.0.0.1:{port}");
    tokio::spawn(async move {
        if let Ok(listener) = TokioTcpListener::bind(&addr).await {
            // Accept connections for up to 5 minutes
            let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(OAUTH_TIMEOUT_SECS));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    Ok((stream, _)) = listener.accept() => {
                        let service = service_fn(handle_oauth_redirect);
                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                                .await;
                        });
                    }
                    () = &mut timeout => break,
                }
            }
        }
    });

    // Store receiver for complete_google_drive_auth to use
    // We'll store it in a separate static for now
    OAUTH_CODE_RECEIVER
        .lock()
        .map_err(|_| "Failed to lock code receiver".to_owned())?
        .replace(rx);

    // 6. Build auth URL
    // Note: This uses hardcoded client ID - in production, load from resources
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "YOUR_CLIENT_ID.apps.googleusercontent.com".to_owned());

    let redirect_uri = format!("http://127.0.0.1:{port}");

    // Build OAuth URL using query parameters
    let params = [
        ("client_id", client_id.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("response_type", "code"),
        ("scope", "https://www.googleapis.com/auth/drive.file https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile"),
        ("state", state.as_str()),
        ("code_challenge", pkce.challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("access_type", "offline"),
        ("prompt", "consent"),
    ];

    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let auth_url = format!("https://accounts.google.com/o/oauth2/v2/auth?{query_string}");

    Ok(OAuthState {
        auth_url,
        server_port: port,
    })
}

#[tauri::command]
pub async fn complete_google_drive_auth(
    db: tauri::State<'_, Database>,
) -> Result<GoogleDriveAccount, String> {
    // 1. Wait for OAuth server to receive code (with timeout)
    let receiver = {
        let mut receiver_guard = OAUTH_CODE_RECEIVER
            .lock()
            .map_err(|_| "Failed to lock code receiver".to_owned())?;

        receiver_guard
            .take()
            .ok_or_else(|| "No OAuth session in progress".to_owned())?
    };

    let code = tokio::time::timeout(
        tokio::time::Duration::from_secs(OAUTH_TIMEOUT_SECS),
        receiver,
    )
    .await
    .map_err(|_| "OAuth timeout - no response received".to_owned())?
    .map_err(|_| "Failed to receive auth code".to_owned())?;

    // 2. Get session data
    let session = {
        let session_guard = OAUTH_SESSION
            .lock()
            .map_err(|_| "Failed to lock OAuth session".to_owned())?;
        session_guard
            .clone()
            .ok_or_else(|| "OAuth session not found".to_owned())?
    };

    // Ensure session cleanup on error via Drop guard
    let _cleanup = SessionCleanup;

    // 3. Exchange code for tokens
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "YOUR_CLIENT_ID.apps.googleusercontent.com".to_owned());
    let client_secret =
        std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| "YOUR_CLIENT_SECRET".to_owned());

    let redirect_uri = format!("http://127.0.0.1:{}", session.port);

    let token_response = exchange_code_for_tokens(
        &code,
        &client_id,
        &client_secret,
        &redirect_uri,
        &session.pkce.verifier,
    )
    .await?;

    // 4. Get user profile
    let user_info = get_user_info(&token_response.access_token).await?;

    // 5. Handle refresh_token - use from response or fall back to existing
    let refresh_token = match token_response.refresh_token {
        Some(token) => token,
        None => {
            // Try to get existing refresh_token from keychain
            get_tokens_from_keychain(&user_info.email)
                .ok()
                .map(|tokens| tokens.refresh_token)
                .ok_or_else(|| "No refresh token available - please reconnect account".to_owned())?
        }
    };

    // 6. Store tokens in keychain
    let token_data = TokenData {
        access_token: token_response.access_token.clone(),
        refresh_token,
        expires_at: Utc::now() + chrono::Duration::seconds(token_response.expires_in),
    };
    store_tokens_in_keychain(&user_info.email, &token_data)?;

    // 7. Save account to database
    let account = GoogleDriveAccount {
        id: uuid::Uuid::new_v4().to_string(),
        email: user_info.email.clone(),
        display_name: user_info.name,
        parent_folder_id: None,
        enabled: true,
        created_at: get_current_timestamp(),
        last_authenticated: get_current_timestamp(),
    };

    db.execute(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO google_drive_accounts \
             (id, email, display_name, parent_folder_id, enabled, created_at, last_authenticated) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                &account.id,
                &account.email,
                &account.display_name,
                &account.parent_folder_id,
                i32::from(account.enabled),
                &account.created_at,
                &account.last_authenticated,
            ],
        )?;
        Ok(())
    })
    .map_err(|e: rusqlite::Error| format!("Failed to save account: {e}"))?;

    // Drop guard will clear OAuth session automatically
    Ok(account)
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

#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
    expires_in: i64,
}

#[allow(dead_code)]
async fn refresh_access_token(refresh_token: &str) -> Result<TokenData, String> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "YOUR_CLIENT_ID.apps.googleusercontent.com".to_owned());
    let client_secret =
        std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| "YOUR_CLIENT_SECRET".to_owned());

    let client = reqwest::Client::new();

    let params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token refresh request failed: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_owned());
        return Err(format!("Token refresh failed: {error_text}"));
    }

    let refresh_response = response
        .json::<RefreshResponse>()
        .await
        .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

    Ok(TokenData {
        access_token: refresh_response.access_token,
        refresh_token: refresh_token.to_owned(),
        expires_at: Utc::now() + chrono::Duration::seconds(refresh_response.expires_in),
    })
}

// Upload Data Structures

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveUploadJob {
    pub id: String,
    pub project_name: String,
    pub folder_name: String,
    pub folder_id: String,
    pub shareable_link: String,
    pub total_files: usize,
    pub uploaded_files: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadProgress {
    job_id: String,
    file_name: String,
    bytes_uploaded: u64,
    total_bytes: u64,
    file_index: usize,
    total_files: usize,
}

// Helper Functions

#[allow(dead_code)]
fn get_current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

/// Get valid access token, refreshing if needed
async fn get_valid_access_token(email: &str) -> Result<String, String> {
    let mut tokens = get_tokens_from_keychain(email)?;

    // Check if token is expired or will expire in next 5 minutes
    let now = Utc::now();
    let buffer = chrono::Duration::minutes(5);

    if tokens.expires_at - buffer < now {
        // Token expired or expiring soon, refresh it
        tokens = refresh_access_token(&tokens.refresh_token).await?;
        store_tokens_in_keychain(email, &tokens)?;
    }

    Ok(tokens.access_token)
}

/// Create folder in Google Drive using REST API
async fn create_drive_folder(
    access_token: &str,
    folder_name: &str,
    parent_folder_id: Option<&str>,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Build folder metadata
    let mut metadata = serde_json::json!({
        "name": folder_name,
        "mimeType": "application/vnd.google-apps.folder"
    });

    if let Some(parent_id) = parent_folder_id {
        metadata["parents"] = serde_json::json!([parent_id]);
    }

    // Create folder via REST API
    let response = client
        .post("https://www.googleapis.com/drive/v3/files")
        .bearer_auth(access_token)
        .json(&metadata)
        .send()
        .await
        .map_err(|e| format!("Failed to create folder: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_owned());
        return Err(format!("Failed to create folder: {error_text}"));
    }

    let folder: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse folder response: {e}"))?;

    folder["id"]
        .as_str()
        .map(std::borrow::ToOwned::to_owned)
        .ok_or_else(|| "Folder created but no ID returned".to_owned())
}

/// Get shareable link for a folder using REST API
async fn get_folder_shareable_link(access_token: &str, folder_id: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Create permission for anyone with link to view
    let permission = serde_json::json!({
        "type": "anyone",
        "role": "reader"
    });

    let response = client
        .post(format!(
            "https://www.googleapis.com/drive/v3/files/{folder_id}/permissions"
        ))
        .bearer_auth(access_token)
        .json(&permission)
        .send()
        .await
        .map_err(|e| format!("Failed to create share permission: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_owned());
        return Err(format!("Failed to create share permission: {error_text}"));
    }

    // Return shareable link
    Ok(format!("https://drive.google.com/drive/folders/{folder_id}"))
}

/// Find existing file in folder by name using REST API
async fn find_existing_file(
    access_token: &str,
    folder_id: &str,
    file_name: &str,
) -> Result<Option<String>, String> {
    let client = reqwest::Client::new();

    // Query for file with matching name in folder
    let query = format!("name = '{file_name}' and '{folder_id}' in parents and trashed = false");

    let response = client
        .get("https://www.googleapis.com/drive/v3/files")
        .bearer_auth(access_token)
        .query(&[("q", &query)])
        .send()
        .await
        .map_err(|e| format!("Failed to search for existing file: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_owned());
        return Err(format!("Failed to search for existing file: {error_text}"));
    }

    let file_list: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse file list: {e}"))?;

    Ok(file_list["files"]
        .as_array()
        .and_then(|files| files.first())
        .and_then(|file| file["id"].as_str())
        .map(std::borrow::ToOwned::to_owned))
}

/// Generate unique filename by adding suffix
fn generate_unique_filename(base_name: &str, extension: &str, attempt: u32) -> String {
    if attempt == 0 {
        format!("{base_name}.{extension}")
    } else {
        format!("{base_name} ({attempt}).{extension}")
    }
}

/// Upload single file to Google Drive with progress tracking using REST API
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
async fn upload_file_to_drive(
    access_token: &str,
    file_path: &str,
    folder_id: &str,
    file_name: &str,
    conflict_mode: &str,
    window: &tauri::Window,
    job_id: &str,
    file_index: usize,
    total_files: usize,
) -> Result<(), String> {
    use std::path::Path;
    use tokio::fs::File as TokioFile;
    use tokio::io::AsyncReadExt;

    // Handle conflict mode
    let final_file_name = match conflict_mode {
        "skip" => {
            // Check if file exists
            if find_existing_file(access_token, folder_id, file_name).await?.is_some() {
                log::info!("Skipping existing file: {file_name}");
                return Ok(());
            }
            file_name.to_owned()
        }
        "rename" => {
            // Find unique name
            let path = Path::new(file_name);
            let base_name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(file_name);
            let extension = path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            let mut attempt = 0;
            let mut unique_name = file_name.to_owned();

            while find_existing_file(access_token, folder_id, &unique_name).await?.is_some() {
                attempt += 1;
                unique_name = generate_unique_filename(base_name, extension, attempt);
                if attempt > 100 {
                    return Err("Failed to find unique filename after 100 attempts".to_owned());
                }
            }
            unique_name
        }
        "overwrite" => {
            // Will upload and overwrite if exists
            file_name.to_owned()
        }
        _ => return Err(format!("Invalid conflict mode: {conflict_mode}")),
    };

    // Read file
    let mut file = TokioFile::open(file_path)
        .await
        .map_err(|e| format!("Failed to open file {file_path}: {e}"))?;

    let file_size = file.metadata()
        .await
        .map_err(|e| format!("Failed to get file metadata: {e}"))?
        .len();

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .await
        .map_err(|e| format!("Failed to read file: {e}"))?;

    // Emit initial progress
    let _ = window.emit(
        "drive-upload-progress",
        UploadProgress {
            job_id: job_id.to_owned(),
            file_name: final_file_name.clone(),
            bytes_uploaded: 0,
            total_bytes: file_size,
            file_index,
            total_files,
        },
    );

    let client = reqwest::Client::new();

    // Check if we need to overwrite existing file
    if conflict_mode == "overwrite" {
        if let Some(existing_id) = find_existing_file(access_token, folder_id, &final_file_name).await? {
            // Update existing file using multipart upload
            let response = client
                .patch(format!(
                    "https://www.googleapis.com/upload/drive/v3/files/{existing_id}?uploadType=media"
                ))
                .bearer_auth(access_token)
                .header("Content-Type", "application/octet-stream")
                .body(buffer)
                .send()
                .await
                .map_err(|e| format!("Failed to overwrite file: {e}"))?;

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_owned());
                return Err(format!("Failed to overwrite file: {error_text}"));
            }

            // Emit completion progress
            let _ = window.emit(
                "drive-upload-progress",
                UploadProgress {
                    job_id: job_id.to_owned(),
                    file_name: final_file_name,
                    bytes_uploaded: file_size,
                    total_bytes: file_size,
                    file_index,
                    total_files,
                },
            );

            return Ok(());
        }
    }

    // Create new file using multipart upload
    let metadata = serde_json::json!({
        "name": final_file_name,
        "parents": [folder_id]
    });

    let boundary = "boundary_string";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{}\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n",
        serde_json::to_string(&metadata).map_err(|e| format!("Failed to serialize metadata: {e}"))?
    );

    let mut body = multipart_body.into_bytes();
    body.extend_from_slice(&buffer);
    body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());

    let response = client
        .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
        .bearer_auth(access_token)
        .header("Content-Type", format!("multipart/related; boundary={boundary}"))
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Failed to upload file: {e}"))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_owned());
        return Err(format!("Failed to upload file: {error_text}"));
    }

    // Emit completion progress
    let _ = window.emit(
        "drive-upload-progress",
        UploadProgress {
            job_id: job_id.to_owned(),
            file_name: final_file_name,
            bytes_uploaded: file_size,
            total_bytes: file_size,
            file_index,
            total_files,
        },
    );

    Ok(())
}

// Upload Tauri Commands

#[tauri::command]
pub async fn upload_to_google_drive(
    window: tauri::Window,
    db: tauri::State<'_, Database>,
    project_name: String,
    files: Vec<String>,
    folder_name: String,
    conflict_mode: String,
) -> Result<DriveUploadJob, String> {
    use tokio::sync::Semaphore;

    // Get account
    let account = get_google_drive_account(db)
        .await?
        .ok_or_else(|| "No Google Drive account configured".to_owned())?;

    if !account.enabled {
        return Err("Google Drive account is disabled".to_owned());
    }

    // Get valid access token
    let access_token = get_valid_access_token(&account.email).await?;

    // Create project folder
    let folder_id = create_drive_folder(
        &access_token,
        &folder_name,
        account.parent_folder_id.as_deref(),
    )
    .await?;

    // Get shareable link
    let shareable_link = get_folder_shareable_link(&access_token, &folder_id).await?;

    // Create job
    let job_id = uuid::Uuid::new_v4().to_string();
    let job = DriveUploadJob {
        id: job_id.clone(),
        project_name: project_name.clone(),
        folder_name: folder_name.clone(),
        folder_id: folder_id.clone(),
        shareable_link,
        total_files: files.len(),
        uploaded_files: 0,
        status: "in_progress".to_owned(),
    };

    // Spawn background task for uploads
    let files_clone = files.clone();
    let conflict_mode_clone = conflict_mode.clone();
    let window_clone = window.clone();
    let access_token_clone = access_token.clone();
    let folder_id_clone = folder_id.clone();

    tokio::spawn(async move {
        // Create semaphore for max 3 concurrent uploads
        let semaphore = Arc::new(Semaphore::new(3));
        let mut tasks = vec![];

        for (index, file_path) in files_clone.iter().enumerate() {
            let Ok(permit) = semaphore.clone().acquire_owned().await else {
                log::error!("Failed to acquire semaphore permit");
                continue;
            };

            let file_path_clone = file_path.clone();
            let access_token_clone2 = access_token_clone.clone();
            let folder_id_clone2 = folder_id_clone.clone();
            let conflict_mode_clone2 = conflict_mode_clone.clone();
            let window_clone2 = window_clone.clone();
            let job_id_clone2 = job_id.clone();
            let total_files = files_clone.len();

            let task = tokio::spawn(async move {
                let _permit = permit;

                // Extract filename
                let file_name = std::path::Path::new(&file_path_clone)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&file_path_clone);

                // Upload with retry (3 attempts)
                let mut attempts = 0;
                let max_attempts = 3;

                loop {
                    attempts += 1;

                    match upload_file_to_drive(
                        &access_token_clone2,
                        &file_path_clone,
                        &folder_id_clone2,
                        file_name,
                        &conflict_mode_clone2,
                        &window_clone2,
                        &job_id_clone2,
                        index,
                        total_files,
                    )
                    .await
                    {
                        Ok(()) => break,
                        Err(e) => {
                            log::error!("Upload attempt {attempts}/{max_attempts} failed for {file_name}: {e}");

                            if attempts >= max_attempts {
                                log::error!("Failed to upload {file_name} after {max_attempts} attempts");
                                break;
                            }

                            // Exponential backoff
                            let delay = std::time::Duration::from_secs(2_u64.pow(attempts - 1));
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            });

            tasks.push(task);
        }

        // Wait for all uploads to complete
        for task in tasks {
            let _ = task.await;
        }

        log::info!("Upload job {job_id} completed");
    });

    Ok(job)
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
    async fn test_start_google_drive_auth_generates_state() {
        let result = start_google_drive_auth().await;
        assert!(result.is_ok());
        let state = result.unwrap();
        assert!(state.auth_url.contains("accounts.google.com"));
        assert!(state.auth_url.contains("code_challenge"));
        assert!(state.server_port > 0);
    }

    // Note: complete_google_drive_auth requires tauri::State which is difficult to mock in tests
    // It will be tested via integration tests or manual testing

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

    #[test]
    fn test_generate_random_alphanumeric_length() {
        let result = generate_random_alphanumeric(64);
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn test_generate_random_alphanumeric_characters() {
        let result = generate_random_alphanumeric(100);
        assert!(result.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_generate_random_alphanumeric_uniqueness() {
        let result1 = generate_random_alphanumeric(50);
        let result2 = generate_random_alphanumeric(50);
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_generate_pkce_verifier_length() {
        let pkce = generate_pkce();
        assert_eq!(pkce.verifier.len(), 128);
    }

    #[test]
    fn test_generate_pkce_challenge_format() {
        let pkce = generate_pkce();
        assert!(!pkce.challenge.is_empty());
        assert!(pkce.challenge.len() > 40);
    }

    #[test]
    fn test_generate_pkce_challenge_base64url() {
        let pkce = generate_pkce();
        assert!(pkce
            .challenge
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert!(!pkce.challenge.contains('='));
        assert!(!pkce.challenge.contains('+'));
        assert!(!pkce.challenge.contains('/'));
    }

    #[test]
    fn test_generate_pkce_uniqueness() {
        let pkce1 = generate_pkce();
        let pkce2 = generate_pkce();
        assert_ne!(pkce1.verifier, pkce2.verifier);
        assert_ne!(pkce1.challenge, pkce2.challenge);
    }

    #[test]
    fn test_generate_state_length() {
        let state = generate_state();
        assert_eq!(state.len(), 32);
    }

    #[test]
    fn test_generate_state_alphanumeric() {
        let state = generate_state();
        assert!(state.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_generate_state_uniqueness() {
        let state1 = generate_state();
        let state2 = generate_state();
        assert_ne!(state1, state2);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_tokens() {
        let test_email = format!("test-{}@example.com", Uuid::new_v4());
        let token_data = TokenData {
            access_token: "test_access".to_owned(),
            refresh_token: "test_refresh".to_owned(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let store_result = store_tokens_in_keychain(&test_email, &token_data);
        assert!(store_result.is_ok());

        if store_result.is_ok() {
            let retrieved = get_tokens_from_keychain(&test_email);
            if let Ok(tokens) = retrieved {
                assert_eq!(tokens.access_token, "test_access");
                assert_eq!(tokens.refresh_token, "test_refresh");
            }

            let entry = keyring::Entry::new("com.creatorops.google-drive", &test_email).unwrap();
            let _ = entry.delete_credential();
        }
    }

    #[tokio::test]
    async fn test_start_google_drive_auth_url_parameters() {
        let result = start_google_drive_auth().await.unwrap();

        assert!(result.auth_url.contains("client_id="));
        assert!(result.auth_url.contains("redirect_uri="));
        assert!(result.auth_url.contains("response_type=code"));
        assert!(result.auth_url.contains("code_challenge="));
        assert!(result.auth_url.contains("code_challenge_method=S256"));
        assert!(result.auth_url.contains("access_type=offline"));
        assert!(result.auth_url.contains("prompt=consent"));
        assert!(result.auth_url.contains("state="));
    }

    #[tokio::test]
    async fn test_start_google_drive_auth_port() {
        let result = start_google_drive_auth().await.unwrap();
        assert_eq!(result.server_port, 8080);
    }

    #[tokio::test]
    async fn test_start_google_drive_auth_creates_session() {
        let _ = start_google_drive_auth().await.unwrap();

        let session_guard = OAUTH_SESSION.lock().unwrap();
        assert!(session_guard.is_some());

        let session = session_guard.as_ref().unwrap();
        assert_eq!(session.port, 8080);
        assert_eq!(session.state.len(), 32);
        assert_eq!(session.pkce.verifier.len(), 128);
        drop(session_guard);
    }

    #[tokio::test]
    async fn test_refresh_response_deserialization() {
        let json = r#"{"access_token":"new_access","expires_in":3600}"#;
        let response: RefreshResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "new_access");
        assert_eq!(response.expires_in, 3600);
    }

    #[tokio::test]
    async fn test_token_response_deserialization() {
        let json = r#"{"access_token":"access","refresh_token":"refresh","expires_in":3600}"#;
        let response: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "access");
        assert_eq!(response.refresh_token, Some("refresh".to_owned()));
        assert_eq!(response.expires_in, 3600);
    }

    #[tokio::test]
    async fn test_token_response_without_refresh() {
        let json = r#"{"access_token":"access","expires_in":3600}"#;
        let response: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "access");
        assert_eq!(response.refresh_token, None);
    }

    #[tokio::test]
    async fn test_user_info_deserialization() {
        let json = r#"{"email":"user@example.com","name":"Test User"}"#;
        let user_info: UserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(user_info.email, "user@example.com");
        assert_eq!(user_info.name, "Test User");
    }

    #[test]
    fn test_session_cleanup_drop() {
        {
            let mut session_guard = OAUTH_SESSION.lock().unwrap();
            *session_guard = Some(OAuthSession {
                pkce: generate_pkce(),
                state: generate_state(),
                port: 8080,
                code_sender: Arc::new(Mutex::new(None)),
            });
        }

        {
            let _cleanup = SessionCleanup;
        }

        assert!(OAUTH_SESSION.lock().unwrap().is_none());
    }

    #[test]
    fn test_pkce_data_clone() {
        let pkce = generate_pkce();
        let cloned = pkce.clone();
        assert_eq!(pkce.verifier, cloned.verifier);
        assert_eq!(pkce.challenge, cloned.challenge);
    }

    #[test]
    fn test_oauth_session_clone() {
        let session = OAuthSession {
            pkce: generate_pkce(),
            state: generate_state(),
            port: 8080,
            code_sender: Arc::new(Mutex::new(None)),
        };

        let cloned = session.clone();
        assert_eq!(session.state, cloned.state);
        assert_eq!(session.port, cloned.port);
    }

    #[test]
    fn test_drive_upload_job_serialization() {
        let job = DriveUploadJob {
            id: "job-123".to_owned(),
            project_name: "Wedding Photos".to_owned(),
            folder_name: "Wedding_2025-01-15".to_owned(),
            folder_id: "folder-abc".to_owned(),
            shareable_link: "https://drive.google.com/drive/folders/folder-abc".to_owned(),
            total_files: 100,
            uploaded_files: 50,
            status: "in_progress".to_owned(),
        };

        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("projectName"));
        assert!(json.contains("folderName"));
        assert!(json.contains("folderId"));
        assert!(json.contains("shareableLink"));
        assert!(json.contains("totalFiles"));
        assert!(json.contains("uploadedFiles"));

        let deserialized: DriveUploadJob = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "job-123");
        assert_eq!(deserialized.total_files, 100);
        assert_eq!(deserialized.uploaded_files, 50);
    }

    #[test]
    fn test_upload_progress_serialization() {
        let progress = UploadProgress {
            job_id: "job-123".to_owned(),
            file_name: "photo.jpg".to_owned(),
            bytes_uploaded: 1024,
            total_bytes: 2048,
            file_index: 5,
            total_files: 10,
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("jobId"));
        assert!(json.contains("fileName"));
        assert!(json.contains("bytesUploaded"));
        assert!(json.contains("totalBytes"));
        assert!(json.contains("fileIndex"));
        assert!(json.contains("totalFiles"));
    }

    #[test]
    fn test_generate_unique_filename_base() {
        let result = generate_unique_filename("photo", "jpg", 0);
        assert_eq!(result, "photo.jpg");
    }

    #[test]
    fn test_generate_unique_filename_with_suffix() {
        let result = generate_unique_filename("photo", "jpg", 1);
        assert_eq!(result, "photo (1).jpg");

        let result = generate_unique_filename("photo", "jpg", 5);
        assert_eq!(result, "photo (5).jpg");
    }

    #[test]
    fn test_generate_unique_filename_empty_extension() {
        let result = generate_unique_filename("document", "", 0);
        assert_eq!(result, "document.");

        let result = generate_unique_filename("document", "", 2);
        assert_eq!(result, "document (2).");
    }
}

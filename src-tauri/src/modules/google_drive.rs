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

// Constants
const MIN_TOKEN_EXPIRY_SECONDS: i64 = 60; // Minimum valid token expiry time
const DEFAULT_TOKEN_EXPIRY_SECONDS: i64 = 3600; // Default 1 hour if invalid expiry received
const HTTP_TIMEOUT_SECONDS: u64 = 60; // HTTP client timeout

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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECONDS))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECONDS)) // Increase timeout
        .connect_timeout(std::time::Duration::from_secs(30)) // Add connection timeout
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    // Retry logic with exponential backoff for network failures (3 total attempts)
    let mut retries = 2;
    let mut delay = std::time::Duration::from_secs(1);

    let response = loop {
        log::info!("Attempting to fetch user info (retries left: {retries})");

        let result = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await;

        match result {
            Ok(response) => {
                log::info!("Successfully fetched user info");
                break response;
            }
            Err(e) if retries > 0 => {
                log::warn!("User info request failed: {e}, retrying in {delay:?}");
                tokio::time::sleep(delay).await;
                retries -= 1;
                delay *= 2; // Exponential backoff
            }
            Err(e) => {
                return Err(format!("User info request failed after retries: {e}"));
            }
        }
    };

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
#[allow(clippy::too_many_lines)]
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

    // Normalize email to lowercase for consistent storage/retrieval
    let normalized_email = user_info.email.to_lowercase();
    log::info!(
        "Authenticated user email (original): '{}', normalized: '{}'",
        user_info.email,
        normalized_email
    );

    // 5. Handle refresh_token - use from response or fall back to existing
    let refresh_token = match token_response.refresh_token {
        Some(token) => token,
        None => {
            // Try to get existing refresh_token from keychain
            get_tokens_from_keychain(&normalized_email)
                .ok()
                .map(|tokens| tokens.refresh_token)
                .ok_or_else(|| "No refresh token available - please reconnect account".to_owned())?
        }
    };

    // 6. Validate and store tokens in keychain
    // Ensure expires_in is reasonable (at least MIN_TOKEN_EXPIRY_SECONDS)
    let expires_in = if token_response.expires_in < MIN_TOKEN_EXPIRY_SECONDS {
        log::warn!(
            "Received unusually short expires_in from Google: {} seconds, using {}",
            token_response.expires_in,
            DEFAULT_TOKEN_EXPIRY_SECONDS
        );
        DEFAULT_TOKEN_EXPIRY_SECONDS
    } else {
        token_response.expires_in
    };

    let token_data = TokenData {
        access_token: token_response.access_token.clone(),
        refresh_token,
        expires_at: Utc::now() + chrono::Duration::seconds(expires_in),
    };
    store_tokens_in_keychain(&normalized_email, &token_data)?;

    // 7. Check if account exists and get its ID, or generate new one
    let existing_account = get_google_drive_account(db.clone()).await?;
    let account_id = existing_account.as_ref().map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |existing| {
            if existing.email.to_lowercase() == normalized_email {
                existing.id.clone()
            } else {
                uuid::Uuid::new_v4().to_string()
            }
        },
    );

    // 8. Save account to database
    let account = GoogleDriveAccount {
        id: account_id.clone(),
        email: normalized_email.clone(),
        display_name: user_info.name,
        parent_folder_id: existing_account.and_then(|a| a.parent_folder_id),
        enabled: true,
        created_at: get_current_timestamp(),
        last_authenticated: get_current_timestamp(),
    };

    log::info!("Saving account to database - ID: '{account_id}', Email: '{normalized_email}'");

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
    .map_err(|e| format!("Failed to save account: {e}"))?;

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
    .map_err(|e| format!("Failed to get account: {e}"))
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
            .map_err(|e| format!("Failed to update parent folder: {e}"))
        },
    )
}

#[tauri::command]
pub async fn remove_google_drive_account(db: tauri::State<'_, Database>) -> Result<(), String> {
    // First get the email to remove from keychain
    let account = get_google_drive_account(db.clone()).await?;

    if let Some(acc) = account {
        // Normalize email for token removal
        let normalized_email = acc.email.to_lowercase();

        // Remove token file
        let token_file = get_token_file_path(&normalized_email).unwrap_or_else(|_| String::new());

        // Ignore error if file doesn't exist
        let _ = std::fs::remove_file(&token_file);

        // Remove from database
        db.execute(|conn| {
            conn.execute("DELETE FROM google_drive_accounts WHERE id = ?1", [&acc.id])?;
            Ok(())
        })
        .map_err(|e| format!("Failed to delete account: {e}"))?;

        log::info!("Removed Google Drive account for {normalized_email}");
    }

    Ok(())
}

#[tauri::command]
pub async fn test_google_drive_connection(db: tauri::State<'_, Database>) -> Result<(), String> {
    let account = get_google_drive_account(db)
        .await?
        .ok_or_else(|| "No Google Drive account configured".to_owned())?;

    log::info!(
        "Testing Google Drive connection for account: '{}' (ID: {})",
        account.email,
        account.id
    );

    // Get valid access token (handles refresh if needed)
    let access_token = get_valid_access_token(&account.email).await.map_err(|e| {
        log::error!("Failed to get valid access token for {}: {}", account.email, e);

        if e.contains("Failed to read token file") || e.contains("No such file") || e.contains("Failed to get tokens") {
            format!("Authentication expired - please disconnect and reconnect your account. (Error: {e})")
        } else if e.contains("Failed to deserialize") || e.contains("Failed to decrypt") {
            format!("Token data corrupted - please disconnect and reconnect your account. (Error: {e})")
        } else {
            e
        }
    })?;

    // Test connection by fetching user info
    log::info!("Testing connection by fetching user info...");
    get_user_info(&access_token).await.map_err(|e| {
        log::error!("Failed to get user info: {e}");
        format!("Connection test failed: {e}")
    })?;

    log::info!("Google Drive connection test successful");
    Ok(())
}

// Token Management Functions

/// Get the token file path for a given email address
fn get_token_file_path(email: &str) -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|_| "Failed to get HOME directory")?;
    let normalized_email = email.to_lowercase();
    Ok(format!(
        "{}/.creatorops/google_tokens_{}.enc",
        home,
        normalized_email.replace('@', "_at_").replace('.', "_")
    ))
}

/// Generate a machine-specific encryption key
#[allow(clippy::unnecessary_wraps)]
fn get_encryption_key() -> Result<[u8; 32], String> {
    use sha2::{Digest, Sha256};

    // Combine multiple machine-specific values for the key
    let mut hasher = Sha256::new();

    // Add username
    if let Ok(user) = std::env::var("USER") {
        hasher.update(user.as_bytes());
    }

    // Add home directory path
    if let Ok(home) = std::env::var("HOME") {
        hasher.update(home.as_bytes());
    }

    // Add hostname if available
    if let Ok(hostname) = std::process::Command::new("hostname").output() {
        hasher.update(&hostname.stdout);
    }

    // Add a fixed salt for this application
    hasher.update(b"CreatorOps-GoogleDrive-TokenEncryption-2024");

    let result = hasher.finalize();
    let mut key = [0_u8; 32];
    key.copy_from_slice(&result);
    Ok(key)
}

/// Encrypt data using AES-256-GCM for secure token storage
/// Uses authenticated encryption with random nonces for each encryption
fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    use aes_gcm::{
        aead::{Aead, AeadCore, KeyInit, OsRng},
        Aes256Gcm,
    };

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("Failed to create cipher: {e}"))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| format!("Failed to encrypt data: {e}"))?;

    // Prepend nonce to ciphertext for storage
    let mut result = nonce.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt data using AES-256-GCM authenticated encryption
/// Validates authenticity and integrity before returning plaintext
fn decrypt_data(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };

    // Extract nonce (first 12 bytes) and ciphertext
    if encrypted.len() < 12 {
        return Err("Invalid encrypted data: too short".to_owned());
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("Failed to create cipher: {e}"))?;

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Failed to decrypt data: {e}"))
}

#[allow(dead_code)]
fn store_tokens_in_keychain(email: &str, tokens: &TokenData) -> Result<(), String> {
    use base64::{engine::general_purpose, Engine as _};

    log::info!("Storing tokens for email: '{email}'");

    // Use encrypted file-based approach as fallback for keychain issues
    let home = std::env::var("HOME").map_err(|_| "Failed to get HOME directory")?;
    let token_dir = format!("{home}/.creatorops");
    std::fs::create_dir_all(&token_dir)
        .map_err(|e| format!("Failed to create token directory: {e}"))?;

    // Set restrictive permissions on the directory (owner only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata =
            std::fs::metadata(&token_dir).map_err(|e| format!("Failed to get metadata: {e}"))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&token_dir, permissions)
            .map_err(|e| format!("Failed to set permissions: {e}"))?;
    }

    let token_file = get_token_file_path(email)?;
    let token_json =
        serde_json::to_string(&tokens).map_err(|e| format!("Failed to serialize tokens: {e}"))?;

    // Encrypt the token data
    let key = get_encryption_key()?;
    let encrypted = encrypt_data(token_json.as_bytes(), &key)?;
    let encoded = general_purpose::STANDARD.encode(&encrypted);

    std::fs::write(&token_file, encoded).map_err(|e| format!("Failed to write token file: {e}"))?;

    // Set restrictive permissions on the file (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata =
            std::fs::metadata(&token_file).map_err(|e| format!("Failed to get metadata: {e}"))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(&token_file, permissions)
            .map_err(|e| format!("Failed to set permissions: {e}"))?;
    }

    log::info!("Successfully stored encrypted tokens for email: '{email}'");
    Ok(())
}

#[allow(dead_code)]
fn get_tokens_from_keychain(email: &str) -> Result<TokenData, String> {
    use base64::{engine::general_purpose, Engine as _};

    log::info!("Attempting to get tokens for email: '{email}'");

    // Use encrypted file-based approach as fallback for keychain issues
    let token_file = get_token_file_path(email)?;

    let encoded = std::fs::read_to_string(&token_file).map_err(|e| {
        log::error!("Failed to read token file for '{email}': {e}");
        format!("Failed to get tokens: {e}")
    })?;

    // Decrypt the token data
    let encrypted = general_purpose::STANDARD
        .decode(&encoded)
        .map_err(|e| format!("Failed to decode token data: {e}"))?;
    let key = get_encryption_key()?;
    let decrypted = decrypt_data(&encrypted, &key)?;
    let token_json = String::from_utf8(decrypted)
        .map_err(|e| format!("Failed to decode decrypted data: {e}"))?;

    let tokens: TokenData = serde_json::from_str(&token_json)
        .map_err(|e| format!("Failed to deserialize tokens: {e}"))?;

    log::info!("Successfully retrieved and decrypted tokens for email: '{email}'");
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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECONDS))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

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

const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB chunks (matches backup.rs pattern)

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
    // Normalize email for consistent keychain access
    let normalized_email = email.to_lowercase();

    let mut tokens = get_tokens_from_keychain(&normalized_email).map_err(|e| {
        log::error!("Failed to get tokens for {normalized_email}: {e}");
        e
    })?;

    // Check if token is expired or will expire in next 5 minutes
    let now = Utc::now();
    let buffer = chrono::Duration::minutes(5);

    if tokens.expires_at - buffer < now {
        log::info!("Token expired or expiring soon for {normalized_email}, refreshing");
        // Token expired or expiring soon, refresh it
        tokens = refresh_access_token(&tokens.refresh_token).await?;
        store_tokens_in_keychain(&normalized_email, &tokens)?;
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
    Ok(format!(
        "https://drive.google.com/drive/folders/{folder_id}"
    ))
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
    email: &str,
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

    // Get fresh access token (handles expiration automatically)
    let access_token = get_valid_access_token(email).await?;

    // Handle conflict mode
    let final_file_name = match conflict_mode {
        "skip" => {
            // Check if file exists
            if find_existing_file(&access_token, folder_id, file_name)
                .await?
                .is_some()
            {
                log::info!("Skipping existing file: {file_name}");
                return Ok(());
            }
            file_name.to_owned()
        }
        "rename" => {
            // Find unique name
            let path = Path::new(file_name);
            let base_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(file_name);
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

            let mut attempt = 0;
            let mut unique_name = file_name.to_owned();

            while find_existing_file(&access_token, folder_id, &unique_name)
                .await?
                .is_some()
            {
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

    // Open file and get metadata
    let mut file = TokioFile::open(file_path)
        .await
        .map_err(|e| format!("Failed to open file {file_path}: {e}"))?;

    let file_size = file
        .metadata()
        .await
        .map_err(|e| format!("Failed to get file metadata: {e}"))?
        .len();

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
    let existing_id = if conflict_mode == "overwrite" {
        find_existing_file(&access_token, folder_id, &final_file_name).await?
    } else {
        None
    };

    // Initiate resumable upload session
    let upload_url = if let Some(existing_id) = existing_id {
        // For updates, use PATCH with uploadType=resumable
        let response = client
            .patch(format!(
                "https://www.googleapis.com/upload/drive/v3/files/{existing_id}?uploadType=resumable"
            ))
            .bearer_auth(&access_token)
            .header("Content-Type", "application/json; charset=UTF-8")
            .send()
            .await
            .map_err(|e| format!("Failed to initiate resumable upload session: {e}"))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            return Err(format!(
                "Failed to initiate resumable upload session: {error_text}"
            ));
        }

        response
            .headers()
            .get("Location")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| "No upload URL in resumable session response".to_owned())?
            .to_owned()
    } else {
        // For new files, use POST with uploadType=resumable
        let metadata = serde_json::json!({
            "name": final_file_name,
            "parents": [folder_id]
        });

        let response = client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable")
            .bearer_auth(&access_token)
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(&metadata)
            .send()
            .await
            .map_err(|e| format!("Failed to initiate resumable upload session: {e}"))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            return Err(format!(
                "Failed to initiate resumable upload session: {error_text}"
            ));
        }

        response
            .headers()
            .get("Location")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| "No upload URL in resumable session response".to_owned())?
            .to_owned()
    };

    // Upload file in chunks
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    let mut bytes_uploaded = 0_u64;

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .await
            .map_err(|e| format!("Failed to read file chunk: {e}"))?;

        if bytes_read == 0 {
            break;
        }

        let chunk_end = bytes_uploaded + bytes_read as u64 - 1;
        let content_range = format!("bytes {bytes_uploaded}-{chunk_end}/{file_size}");

        let response = client
            .put(&upload_url)
            .header("Content-Length", bytes_read.to_string())
            .header("Content-Range", content_range)
            .body(buffer[..bytes_read].to_vec())
            .send()
            .await
            .map_err(|e| format!("Failed to upload chunk: {e}"))?;

        if !response.status().is_success() && response.status().as_u16() != 308 {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            return Err(format!("Failed to upload chunk: {error_text}"));
        }

        bytes_uploaded += bytes_read as u64;

        // Emit progress after each chunk
        let _ = window.emit(
            "drive-upload-progress",
            UploadProgress {
                job_id: job_id.to_owned(),
                file_name: final_file_name.clone(),
                bytes_uploaded,
                total_bytes: file_size,
                file_index,
                total_files,
            },
        );
    }

    Ok(())
}

// Upload Tauri Commands

#[tauri::command]
#[allow(clippy::too_many_lines)]
pub async fn upload_to_google_drive(
    window: tauri::Window,
    db: tauri::State<'_, Database>,
    project_name: String,
    files: Vec<String>,
    folder_name: String,
    conflict_mode: String,
) -> Result<DriveUploadJob, String> {
    use tokio::sync::Semaphore;

    // Validate file paths
    for file_path in &files {
        if !std::path::Path::new(file_path).exists() {
            return Err(format!("File not found: {file_path}"));
        }
        if !std::path::Path::new(file_path).is_file() {
            return Err(format!("Not a file: {file_path}"));
        }
    }

    // Get account
    let account = get_google_drive_account(db)
        .await?
        .ok_or_else(|| "No Google Drive account configured".to_owned())?;

    if !account.enabled {
        return Err("Google Drive account is disabled".to_owned());
    }

    // Get valid access token for initial folder creation
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
    let email_clone = account.email.clone();
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
            let email_clone2 = email_clone.clone();
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
                        &email_clone2,
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
                                log::error!(
                                    "Failed to upload {file_name} after {max_attempts} attempts"
                                );
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
                Ok(stmt
                    .query_row(["account-1"], |row| {
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
                    .optional()?)
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
                Ok(stmt
                    .query_row(["account-2"], |row| {
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
                    .optional()?)
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
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM google_drive_accounts WHERE id = ?1",
                    ["remove-id"],
                    |row| row.get(0),
                )?)
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
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM google_drive_accounts WHERE id = ?1",
                    ["remove-id"],
                    |row| row.get(0),
                )?)
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
        assert!(result.unwrap_err().contains("Failed to get tokens"));
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

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42_u8; 32]; // Test key
        let plaintext = b"This is a test message for encryption";

        // Encrypt
        let encrypted = encrypt_data(plaintext, &key).unwrap();

        // Verify encrypted is different from plaintext
        assert_ne!(&encrypted[12..], plaintext);

        // Decrypt
        let decrypted = decrypt_data(&encrypted, &key).unwrap();

        // Verify roundtrip
        assert_eq!(&decrypted[..], plaintext);
    }

    #[test]
    fn test_encrypt_different_nonces() {
        let key = [42_u8; 32];
        let plaintext = b"Test message";

        let encrypted1 = encrypt_data(plaintext, &key).unwrap();
        let encrypted2 = encrypt_data(plaintext, &key).unwrap();

        // Same plaintext should produce different ciphertexts (different nonces)
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same plaintext
        let decrypted1 = decrypt_data(&encrypted1, &key).unwrap();
        let decrypted2 = decrypt_data(&encrypted2, &key).unwrap();
        assert_eq!(decrypted1, decrypted2);
        assert_eq!(&decrypted1[..], plaintext);
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let key = [42_u8; 32];

        // Too short (less than nonce size)
        let short_data = vec![1, 2, 3];
        let result = decrypt_data(&short_data, &key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));

        // Invalid ciphertext (random data)
        let mut invalid_data = vec![0_u8; 50];
        invalid_data[..12].copy_from_slice(&[1_u8; 12]); // Valid nonce size
        let result = decrypt_data(&invalid_data, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_token_file_path() {
        let email = "test@example.com";
        let path = get_token_file_path(email).unwrap();

        assert!(path.ends_with("/.creatorops/google_tokens_test_at_example_com.enc"));
        assert!(path.contains(std::env::var("HOME").unwrap_or_default().as_str()));
    }

    #[test]
    fn test_get_token_file_path_normalization() {
        let email1 = "Test@Example.Com";
        let email2 = "test@example.com";

        let path1 = get_token_file_path(email1).unwrap();
        let path2 = get_token_file_path(email2).unwrap();

        // Both should produce same path (normalized)
        assert_eq!(path1, path2);
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

    #[test]
    fn test_chunk_size_constant() {
        assert_eq!(CHUNK_SIZE, 4 * 1024 * 1024);
        assert_eq!(CHUNK_SIZE, 4_194_304);
    }

    #[test]
    fn test_chunk_size_matches_backup_module() {
        // Verify CHUNK_SIZE matches the pattern used in backup.rs
        const BACKUP_CHUNK_SIZE: usize = 4 * 1024 * 1024;
        assert_eq!(CHUNK_SIZE, BACKUP_CHUNK_SIZE);
    }

    #[test]
    fn test_upload_progress_complete_struct() {
        let progress = UploadProgress {
            job_id: "job-123".to_owned(),
            file_name: "photo.jpg".to_owned(),
            bytes_uploaded: 2048,
            total_bytes: 2048,
            file_index: 10,
            total_files: 100,
        };

        assert_eq!(progress.job_id, "job-123");
        assert_eq!(progress.file_name, "photo.jpg");
        assert_eq!(progress.bytes_uploaded, 2048);
        assert_eq!(progress.total_bytes, 2048);
        assert_eq!(progress.file_index, 10);
        assert_eq!(progress.total_files, 100);
    }

    #[test]
    fn test_upload_progress_partial() {
        let progress = UploadProgress {
            job_id: "job-456".to_owned(),
            file_name: "video.mp4".to_owned(),
            bytes_uploaded: 1024,
            total_bytes: 4096,
            file_index: 5,
            total_files: 20,
        };

        assert!(progress.bytes_uploaded < progress.total_bytes);
        assert_eq!(progress.bytes_uploaded, 1024);
        assert_eq!(progress.total_bytes, 4096);
    }

    #[test]
    fn test_drive_upload_job_status_values() {
        let job = DriveUploadJob {
            id: "test-id".to_owned(),
            project_name: "Test".to_owned(),
            folder_name: "Test Folder".to_owned(),
            folder_id: "folder-123".to_owned(),
            shareable_link: "https://drive.google.com/drive/folders/folder-123".to_owned(),
            total_files: 10,
            uploaded_files: 5,
            status: "in_progress".to_owned(),
        };

        assert_eq!(job.status, "in_progress");
        assert!(job.uploaded_files < job.total_files);
    }

    #[test]
    fn test_drive_upload_job_with_zero_files() {
        let job = DriveUploadJob {
            id: "empty-job".to_owned(),
            project_name: "Empty".to_owned(),
            folder_name: "Empty Folder".to_owned(),
            folder_id: "folder-empty".to_owned(),
            shareable_link: "https://drive.google.com/drive/folders/folder-empty".to_owned(),
            total_files: 0,
            uploaded_files: 0,
            status: "completed".to_owned(),
        };

        assert_eq!(job.total_files, 0);
        assert_eq!(job.uploaded_files, 0);
        assert_eq!(job.status, "completed");
    }

    #[test]
    fn test_generate_unique_filename_sequential() {
        let base = "photo";
        let ext = "jpg";

        assert_eq!(generate_unique_filename(base, ext, 0), "photo.jpg");
        assert_eq!(generate_unique_filename(base, ext, 1), "photo (1).jpg");
        assert_eq!(generate_unique_filename(base, ext, 2), "photo (2).jpg");
        assert_eq!(generate_unique_filename(base, ext, 99), "photo (99).jpg");
    }

    #[test]
    fn test_generate_unique_filename_special_characters() {
        let base = "my-photo_2024";
        let ext = "jpeg";

        assert_eq!(generate_unique_filename(base, ext, 0), "my-photo_2024.jpeg");
        assert_eq!(
            generate_unique_filename(base, ext, 1),
            "my-photo_2024 (1).jpeg"
        );
    }

    #[test]
    fn test_store_and_get_tokens_roundtrip() {
        let email = format!("test-{}@example.com", Uuid::new_v4());
        let tokens = TokenData {
            access_token: "test-access-token-12345".to_owned(),
            refresh_token: "test-refresh-token-67890".to_owned(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        // Store tokens
        let store_result = store_tokens_in_keychain(&email, &tokens);
        assert!(
            store_result.is_ok(),
            "Failed to store tokens: {:?}",
            store_result.err()
        );

        // Retrieve tokens
        let retrieved_result = get_tokens_from_keychain(&email);
        assert!(
            retrieved_result.is_ok(),
            "Failed to retrieve tokens: {:?}",
            retrieved_result.err()
        );

        let retrieved = retrieved_result.unwrap();
        assert_eq!(retrieved.access_token, tokens.access_token);
        assert_eq!(retrieved.refresh_token, tokens.refresh_token);

        // Clean up
        let token_file = get_token_file_path(&email).unwrap();
        let _ = std::fs::remove_file(token_file);
    }

    #[test]
    fn test_store_tokens_creates_directory() {
        let email = format!("test-dir-{}@example.com", Uuid::new_v4());
        let tokens = TokenData {
            access_token: "test-token".to_owned(),
            refresh_token: "refresh-token".to_owned(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let result = store_tokens_in_keychain(&email, &tokens);
        assert!(result.is_ok());

        // Check that the .creatorops directory exists
        let home = std::env::var("HOME").unwrap();
        let token_dir = format!("{home}/.creatorops");
        assert!(std::path::Path::new(&token_dir).exists());

        // Clean up
        let token_file = get_token_file_path(&email).unwrap();
        let _ = std::fs::remove_file(token_file);
    }

    #[test]
    fn test_get_tokens_handles_corrupted_file() {
        let email = format!("test-corrupt-{}@example.com", Uuid::new_v4());

        // Create a corrupted token file
        let token_file = get_token_file_path(&email).unwrap();
        let home = std::env::var("HOME").unwrap();
        let token_dir = format!("{home}/.creatorops");
        let _ = std::fs::create_dir_all(&token_dir);

        // Write invalid base64 data
        std::fs::write(&token_file, "not-valid-base64!@#$%").unwrap();

        let result = get_tokens_from_keychain(&email);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to decode token data"));

        // Clean up
        let _ = std::fs::remove_file(token_file);
    }

    #[test]
    fn test_get_tokens_handles_invalid_encrypted_data() {
        use base64::{engine::general_purpose, Engine as _};

        let email = format!("test-invalid-{}@example.com", Uuid::new_v4());

        // Create a file with valid base64 but invalid encrypted content
        let token_file = get_token_file_path(&email).unwrap();
        let home = std::env::var("HOME").unwrap();
        let token_dir = format!("{home}/.creatorops");
        let _ = std::fs::create_dir_all(&token_dir);
        // Create random data that's not properly encrypted
        let invalid_data = vec![0_u8; 100];
        let encoded = general_purpose::STANDARD.encode(&invalid_data);
        std::fs::write(&token_file, encoded).unwrap();

        let result = get_tokens_from_keychain(&email);
        assert!(result.is_err());

        // Clean up
        let _ = std::fs::remove_file(token_file);
    }

    #[test]
    fn test_encryption_key_consistency() {
        // The encryption key should be consistent for the same machine
        let key1 = get_encryption_key().unwrap();
        let key2 = get_encryption_key().unwrap();
        assert_eq!(key1, key2, "Encryption key should be consistent");
    }

    #[test]
    fn test_encryption_key_length() {
        let key = get_encryption_key().unwrap();
        assert_eq!(key.len(), 32, "Encryption key must be 32 bytes for AES-256");
    }

    #[test]
    fn test_encrypt_decrypt_empty_data() {
        let key = get_encryption_key().unwrap();
        let data = b"";

        let encrypted = encrypt_data(data, &key).unwrap();
        assert!(
            encrypted.len() >= 12,
            "Encrypted data should at least contain nonce"
        );

        let decrypted = decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let key = get_encryption_key().unwrap();
        let data = vec![42_u8; 10000]; // 10KB of data

        let encrypted = encrypt_data(&data, &key).unwrap();
        let decrypted = decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let key1 = [1_u8; 32];
        let key2 = [2_u8; 32];
        let data = b"secret data";

        let encrypted = encrypt_data(data, &key1).unwrap();
        let result = decrypt_data(&encrypted, &key2);

        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    fn test_token_file_path_sanitization() {
        // Test that email addresses are properly sanitized for file paths
        let email1 = "user@example.com";
        let email2 = "USER@EXAMPLE.COM";
        let email3 = "User@Example.Com";

        let path1 = get_token_file_path(email1).unwrap();
        let path2 = get_token_file_path(email2).unwrap();
        let path3 = get_token_file_path(email3).unwrap();

        // All should normalize to the same path
        assert_eq!(path1, path2);
        assert_eq!(path2, path3);

        // Should sanitize email special characters in the filename
        assert!(
            path1.contains("google_tokens_user_at_example_com.enc"),
            "Should have sanitized filename"
        );
        assert!(
            path1.ends_with("user_at_example_com.enc"),
            "Should end with sanitized email"
        );

        // Check the filename part doesn't contain raw email characters
        let filename = path1.rsplit('/').next().unwrap();
        assert!(!filename.contains('@'), "Filename should not contain raw @");
        assert!(
            filename.contains("_at_"),
            "Filename should replace @ with _at_"
        );
    }

    #[test]
    fn test_store_tokens_with_special_characters() {
        let email = format!("test+special.chars{}@example.com", Uuid::new_v4());
        let tokens = TokenData {
            access_token: "token-with-special-chars!@#$%^&*()".to_owned(),
            refresh_token: "refresh-with-unicode-émojis-🎉".to_owned(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let result = store_tokens_in_keychain(&email, &tokens);
        assert!(result.is_ok());

        let retrieved = get_tokens_from_keychain(&email).unwrap();
        assert_eq!(retrieved.access_token, tokens.access_token);
        assert_eq!(retrieved.refresh_token, tokens.refresh_token);

        // Clean up
        let token_file = get_token_file_path(&email).unwrap();
        let _ = std::fs::remove_file(token_file);
    }

    #[test]
    #[allow(clippy::significant_drop_tightening)]
    fn test_concurrent_token_storage() {
        use std::sync::Arc;
        use std::sync::Mutex;
        use std::thread;

        let email = format!("test-concurrent-{}@example.com", Uuid::new_v4());
        let results = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];

        for i in 0..5 {
            let email_clone = email.clone();
            let results_clone = results.clone();

            let handle = thread::spawn(move || {
                let tokens = TokenData {
                    access_token: format!("token-{i}"),
                    refresh_token: format!("refresh-{i}"),
                    expires_at: Utc::now() + chrono::Duration::hours(1),
                };

                let result = store_tokens_in_keychain(&email_clone, &tokens);
                results_clone.lock().unwrap().push(result.is_ok());
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        {
            let all_results = results.lock().unwrap();
            assert!(
                all_results.iter().all(|&r| r),
                "All concurrent stores should succeed"
            );
        } // Drop the lock guard here

        // Verify we can still read tokens
        let final_tokens = get_tokens_from_keychain(&email);
        assert!(final_tokens.is_ok());

        // Clean up
        let token_file = get_token_file_path(&email).unwrap();
        let _ = std::fs::remove_file(token_file);
    }
}

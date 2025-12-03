# Google Drive Integration for Gallery Delivery

## Objective
Add Google Drive as delivery destination alongside existing local filesystem destinations. Users can upload selected project files to Drive with progress tracking.

## Recommended Approach

### Architecture: Extend Existing Delivery System
**Rationale**: Leverage proven delivery queue pattern. Minimal disruption, consistent UX.

**Key Decision**: Make `DeliveryDestination` polymorphic - support both Local and GoogleDrive types via Rust enum with serde tag.

### OAuth Flow: Loopback Server (RFC 8252)
**Method**:
1. User clicks "Connect Google Drive" in Settings
2. Spawn localhost HTTP server on ephemeral port
3. Open browser to Google OAuth consent
4. Capture auth code from redirect to `http://localhost:PORT`
5. Exchange code for tokens (with PKCE)
6. Store tokens in system keychain
7. Shutdown server

**Rationale**: Standard for desktop apps. No external redirect URI needed. PKCE prevents interception.

### Token Storage: System Keychain
**Library**: `keyring` crate (macOS Keychain, Windows Credential Manager, Linux Secret Service)

**Key**: `service: com.creatorops.google-drive, account: user@gmail.com`

**Value**: JSON with `{access_token, refresh_token, expires_at}`

**Rationale**: OS-level security. Never in database/localStorage/files.

### Upload Strategy: Resumable Uploads
**Library**: `google-drive3` crate (official Google API)

**Implementation**:
- Chunked streaming (8MB chunks per Google recommendation)
- Progress events per chunk → frontend progress bars
- Exponential backoff retry for network errors
- 3 parallel uploads max (avoid rate limits)

**Folder Structure**: User-configurable parent folder with project subfolders
- User selects parent folder location in Settings (can be Drive root or any folder)
- Each delivery creates subfolder: `ParentFolder/ProjectName_Date/`
- Example: `My Deliveries/Wedding_Smith_2025-11-28/` contains all delivered files
- Respects naming templates from frontend

### Single Account (MVP)
**Storage**: SQLite table `google_drive_accounts` (supports single account initially)
**Token**: Keychain entry for authenticated account
**UI**: Connect/disconnect in Settings

## Implementation Plan

### Phase 1: Backend Foundation

**1. New Module: `src-tauri/src/modules/google_drive.rs`**
- OAuth functions: `start_auth()`, `complete_auth()`, `refresh_token()`
- Account functions: `get_account()`, `remove_account()`
- Token management: `get_tokens_from_keychain()`, `store_tokens()`
- Upload functions: `create_folder()`, `upload_file()`, `upload_with_progress()`

**Tests**:
- Unit tests for PKCE generation/verification
- Token refresh logic tests
- Mock OAuth server for testing flow

**2. Extend: `src-tauri/src/modules/delivery.rs`**
```rust
// Change DeliveryDestination from String to enum
#[serde(tag = "type")]
pub enum DeliveryDestination {
    Local { id, name, path, enabled, created_at },
    GoogleDrive { id, name, account_id, folder_id, enabled, created_at }
}

// Update create_delivery() to dispatch based on destination type
match destination {
    Local => create_local_delivery(...),
    GoogleDrive => create_drive_delivery(...)
}
```

**Tests**:
- Unit tests for destination type serialization/deserialization
- Test dispatch logic for both destination types

**3. Database Schema: `src-tauri/src/modules/db.rs`**
```sql
CREATE TABLE google_drive_accounts (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    last_authenticated TEXT NOT NULL
);
```

**4. Dependencies (Cargo.toml)**
```toml
google-drive3 = "6.0.0"       # Google Drive API
yup-oauth2 = "11"              # OAuth2 authenticator
hyper = "1.0"                  # HTTP (required by google-drive3)
hyper-rustls = "0.27"          # TLS
keyring = "3.0"                # Secure credential storage
```

### Phase 2: Frontend Integration

**1. Type Definitions: `src/types/index.ts`**
```typescript
export interface GoogleDriveAccount {
  id: string
  email: string
  displayName: string
  enabled: boolean
  createdAt: string
  lastAuthenticated: string
}

export type DeliveryDestination =
  | { type: 'local', id, name, path, enabled, createdAt }
  | { type: 'google-drive', id, name, accountId, folderId?, enabled, createdAt }
```

**2. Settings Component: `src/components/Settings.tsx`**
- Add "Google Drive Integration" section
- Show connected account (single account MVP)
- "Connect Google Drive" button → triggers OAuth flow
- "Configure Parent Folder" button → sets parent folder ID
- "Disconnect" button → deletes account + keychain tokens
- "Test Connection" button → validates tokens still work
- Add "File Conflict Handling" setting:
  - Radio buttons: Overwrite / Rename / Skip
  - Store in localStorage: `drive_conflict_mode`
  - Default: 'rename'

**Tests**:
- Component tests for OAuth flow UI
- Test localStorage migration
- Test conflict mode setting persistence

**3. Delivery Component: `src/components/Delivery.tsx`**
- Load Drive account as destination (alongside local)
- Show cloud icon for Drive destination
- Handle Drive upload progress events
- Display Drive shareable link on completion with "Copy Link" button
- Show folder structure: `ParentFolder/ProjectName_Date/`

**Tests**:
- Test Drive destination rendering
- Test shareable link copy functionality
- Test progress event handling

**4. localStorage Migration**
Migrate existing `delivery_destinations` from string paths to typed objects with `type: 'local'`.

### Phase 3: OAuth Implementation Details

**Backend Commands**:
```rust
#[tauri::command]
async fn start_google_drive_auth() -> Result<OAuthState, String>
// Returns: { auth_url: "https://accounts.google.com/...", server_port: 8080 }

#[tauri::command]
async fn complete_google_drive_auth() -> Result<GoogleDriveAccount, String>
// Waits for OAuth redirect, exchanges code, stores tokens, returns account

#[tauri::command]
async fn get_google_drive_account() -> Result<Option<GoogleDriveAccount>, String>
// Get connected account if exists

#[tauri::command]
async fn set_drive_parent_folder(folder_id: Option<String>) -> Result<(), String>
// Updates parent_folder_id in database

#[tauri::command]
async fn remove_google_drive_account() -> Result<(), String>
// Remove account and keychain tokens

#[tauri::command]
async fn upload_to_google_drive(
    project_name: String,
    files: Vec<String>,
    folder_name: String,
    conflict_mode: String  // "overwrite" | "rename" | "skip"
) -> Result<DriveUploadJob, String>
// Returns job with shareable_link field

#[tauri::command]
async fn get_drive_folder_shareable_link(folder_id: String) -> Result<String, String>
// Generates shareable link for folder
```

**Tests**:
- Integration tests for each command
- Mock Google Drive API responses

**Frontend Flow**:
```typescript
// Settings.tsx
async function handleConnectDrive() {
  const { authUrl } = await invoke('start_google_drive_auth')
  await open(authUrl)  // Open browser
  const account = await invoke('complete_google_drive_auth')
  // Success - account connected
}
```

### Phase 4: Upload Implementation

**Chunked Upload with Progress**:
```rust
// Wrap file reader with ProgressTracker
struct ProgressTracker<R: AsyncRead> {
    inner: R,
    bytes_read: Arc<AtomicU64>,
    app_handle: AppHandle,
    job_id: String,
}

// Emit event every 4MB chunk
impl AsyncRead for ProgressTracker {
    fn poll_read(...) {
        // Read chunk
        // Update bytes_read
        // Emit 'drive-upload-progress' event
    }
}
```

**Parallel Uploads**:
- Use tokio Semaphore with limit of 3 concurrent uploads
- Prevents rate limiting
- Better utilization of bandwidth

**Error Handling**:
- Network errors → exponential backoff retry (3 attempts)
- Auth errors → prompt re-authentication
- Quota exceeded → stop, notify user
- Rate limited → wait, then retry
- File exists (based on conflict_mode):
  - "overwrite": Replace existing file
  - "rename": Add suffix (photo.jpg → photo (1).jpg)
  - "skip": Skip file, log warning, continue

**Tests**:
- Unit tests for ProgressTracker
- Test retry logic with mock failures
- Test conflict mode handling (overwrite/rename/skip)
- Integration test with real Drive API (manual, using test account)

### Phase 5: End-to-End Testing

**Manual Testing Checklist**:
- Connect/disconnect account
- Upload single file, many files, large files
- Cancel mid-upload
- Token expiration after 1 hour
- Network interruption recovery
- All conflict modes (overwrite/rename/skip)
- Shareable link generation and copy

**Note**: Unit and integration tests written throughout each phase (see tests sections above)

## Security

**Token Security**:
- ✅ Store in system keychain only
- ✅ Never log tokens
- ✅ Auto-refresh before expiry
- ✅ Handle revocation gracefully

**OAuth Best Practices**:
- ✅ Use PKCE (prevents authorization code interception)
- ✅ Loopback server on localhost (RFC 8252)
- ✅ Minimal scope (`drive.file` not `drive`)
- ✅ State parameter (CSRF protection)

**Client Secret**:
- Desktop apps are "public clients" per OAuth spec
- Bundle client secret in app resources (acceptable)
- PKCE compensates for inability to keep secret truly secret

## Critical Files

**To Create**:
- `src-tauri/src/modules/google_drive.rs` - OAuth + Drive API (new module)
- `src-tauri/resources/google_client_secret.json` - OAuth credentials (bundled)

**To Modify**:
- `src-tauri/src/modules/delivery.rs` - Add Drive destination type, conflict handling
- `src-tauri/src/modules/db.rs` - Add google_drive_accounts table with parent_folder_id
- `src-tauri/src/modules/mod.rs` - Register new commands
- `src-tauri/Cargo.toml` - Add dependencies
- `src/components/Settings.tsx` - Add Drive section + conflict mode setting
- `src/components/Delivery.tsx` - Support Drive destinations, show shareable link
- `src/types/index.ts` - Add Drive types
- `src/styles/components.css` - Style Drive UI (link copy button, folder path)

## Rollout Strategy

**MVP**:
- Single account support
- User-configurable parent folder location
- Upload creates project subfolders within parent
- Progress tracking with real-time updates
- Shareable link generation + copy button
- Conflict handling (overwrite/rename/skip)
- Settings UI for auth
- Delivery UI supports Drive alongside local
- Tests written throughout each phase


## Design Decisions (Confirmed)

1. **Folder Organization**: Single parent folder with project subfolders
   - User configures parent folder location in Settings (e.g., "CreatorOps Deliveries")
   - Each delivery creates subfolder: `ParentFolder/ProjectName_Date/`
   - Parent folder setting stored per Drive account

2. **Client Sharing**:
   - **MVP**: Generate shareable link + copy to clipboard button
   - **Future**: Auto-share with client email (requires client email in project model)

3. **File Naming Conflicts**: User-configurable setting
   - Add option in Settings: "On file conflict: Overwrite / Rename / Skip"
   - Default: Rename with suffix (`photo (1).jpg`)

4. **OAuth Client Secret**: Bundle in app
   - Include `client_secret.json` in `src-tauri/resources/`
   - Load at runtime, never expose to frontend

## Future Enhancements (Post-MVP)

- Multiple account support
- Folder picker (browse Drive folders to select parent)
- Auto-share folders with client email
- Upload resume after crash/network interruption
- Bandwidth throttling
- Storage quota warnings before upload
- Offline queue (upload when connection restored)
- Large file optimization (>1GB chunking strategy)
- Upload history/analytics

## Estimated Effort

**MVP**: 2-3 weeks
- Backend OAuth + upload + tests: 1 week
- Frontend integration + tests: 1 week
- E2E testing + debugging: 0.5-1 week

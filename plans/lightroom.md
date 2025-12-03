# Lightroom Classic Export Integration Implementation Plan

## Goal

Enable CreatorOps to trigger Lightroom Classic collection exports via Lua plugin using Unix socket communication.

## Architecture: Unix Socket Bridge ⭐

**Why Unix sockets:**

- Real-time bidirectional communication (no polling)
- No port conflicts or firewall issues
- Faster than TCP/HTTP (local kernel IPC)
- Clean request/response pattern
- Simple error handling

**Communication flow:**

1. Lightroom plugin creates Unix socket at `~/.creatorops/lightroom.sock`
2. CreatorOps connects to socket, sends JSON export request
3. Plugin validates request, starts export, responds immediately
4. Plugin sends progress updates over same connection
5. CreatorOps receives completion status

## Technical Stack

**Lightroom Plugin (Lua):**

- LuaSocket library (included in SDK)
- Unix domain socket server
- `LrExportSession` API for exports
- `LrCatalog` API for collection lookup
- Timer-based socket accept loop

**CreatorOps (Rust):**

- tokio UnixStream for socket client
- serde_json for request/response
- Existing Tauri event emitter for progress
- New module: `src-tauri/src/modules/lightroom.rs`

**User Configuration (Settings):**

- Export presets: JPEG quality, destination mapping
- Auto-match project name to collection (toggle)
- Default destination: Selects folder

## Implementation Phases

### Phase 1: Lightroom Plugin Development

**Setup:**

- Download Lightroom Classic SDK from Adobe Developer Portal
- Review SDK docs + LuaSocket examples
- Create plugin dev environment

**Plugin structure (`CreatorOpsExporter.lrplugin/`):**

```text
Info.lua              - Plugin metadata, SDK version
SocketServer.lua      - Unix socket server
ExportHandler.lua     - Export execution logic
```

**Core implementation:**

1. **Info.lua** - Plugin manifest
   - SDK version compatibility
   - Required permissions
   - Plugin metadata

2. **SocketServer.lua** - Socket communication (~150 lines)
   - Create Unix socket at `~/.creatorops/lightroom.sock`
   - Timer-based accept loop (100ms intervals)
   - Parse JSON requests: `{collection, preset, destination, projectId}`
   - Call ExportHandler, send JSON response
   - Handle errors, connection cleanup

3. **ExportHandler.lua** - Export logic (~200 lines)
   - Lookup collection by name (exact match or auto-match project)
   - Load export settings from preset name
   - Map preset to destination (Selects, Delivery, etc.)
   - Create `LrExportSession` with settings
   - Send progress updates over socket
   - Return completion status

**Export presets (defined in plugin):**

- `selects-jpeg-full` → JPEG 100%, sRGB, 6000px long edge → Selects/
- `selects-jpeg-web` → JPEG 90%, sRGB, 2048px → Selects/Web/
- `delivery-jpeg` → JPEG 95%, sRGB, full res → Delivery/
- `delivery-tiff` → TIFF 16-bit, Adobe RGB → Delivery/TIFF/

**Installation:**

- User copies `.lrplugin` folder to Lightroom modules
- Enable in Plugin Manager
- Plugin auto-starts on Lightroom launch
- Creates socket automatically

### Phase 2: CreatorOps Backend Integration

**New module: `src-tauri/src/modules/lightroom.rs`**

Key functions:

```rust
// Connect to LR socket, send export request, receive response
#[tauri::command]
async fn trigger_lightroom_export(
    project_id: String,
    collection_name: Option<String>,
    preset_name: String,
    destination: String,
) -> Result<LightroomExportResponse, String>

// Check if Lightroom plugin is running
#[tauri::command]
async fn check_lightroom_connection() -> Result<bool, String>
```

**Implementation details:**

- Use `tokio::net::UnixStream` to connect to socket
- Serialize request as JSON: `{collection, preset, destination, projectId}`
- Send request, read response (with 30s timeout)
- Parse response: `{status, filesExported, error?}`
- Emit Tauri events for progress if LR sends updates
- Handle connection errors gracefully

**Auto-match logic:**

- If `collection_name` is None, use project name
- Plugin tries exact match first
- If not found, try fuzzy match (project date + client name)
- Return error if no match found

### Phase 3: Frontend Implementation

**Settings UI changes (`src/components/Settings.tsx`):**

Add "Lightroom Integration" section:

- Toggle: "Enable Lightroom Export"
- Collection auto-match toggle (default: on)
- Export preset mapping:
  - Destination: Selects → Preset: [dropdown]
  - Destination: Delivery → Preset: [dropdown]
- Connection status indicator (green dot if connected)
- Installation instructions link

**Projects UI changes (`src/components/Projects.tsx`):**

Add export button in project actions:

- Button: "Export from Lightroom" (only if LR connected)
- Click opens modal with:
  - Collection name (pre-filled with project name, editable)
  - Destination: Selects / Delivery (radio buttons)
  - Preset: auto-selected from settings mapping
- Progress modal during export
- Success notification with file count

**Type definitions (`src/types/index.ts`):**

```typescript
interface LightroomExportRequest {
  projectId: string
  collectionName?: string
  presetName: string
  destination: string
}

interface LightroomExportResponse {
  status: 'success' | 'error'
  filesExported?: number
  error?: string
}

interface LightroomSettings {
  enabled: boolean
  autoMatchCollection: boolean
  presetMapping: {
    selects: string
    delivery: string
  }
}
```

### Phase 4: Testing

**Manual test scenarios:**

1. Happy path: Export collection to Selects
2. Auto-match: Create project, trigger export without collection name
3. LR not running: Graceful error message
4. Collection not found: Clear error, suggest alternatives
5. Invalid preset: Fallback to default
6. Concurrent exports: Queue handling
7. Socket permission errors: Clear troubleshooting

**Edge cases:**

- Socket file already exists (stale from crash)
- Lightroom quit during export
- Disk full during export
- Very large collections (1000+ photos)

## Critical Files

**New files:**

- `src-tauri/src/modules/lightroom.rs` - Socket client, export commands
- `CreatorOpsExporter.lrplugin/Info.lua` - Plugin manifest
- `CreatorOpsExporter.lrplugin/SocketServer.lua` - Socket server
- `CreatorOpsExporter.lrplugin/ExportHandler.lua` - Export logic

**Modified files:**

- `src/components/Settings.tsx` - LR integration settings
- `src/components/Projects.tsx` - Export button + modal
- `src/types/index.ts` - LR type definitions
- `src-tauri/src/main.rs` - Register LR commands

**Reference files:**

- `src-tauri/src/modules/file_system.rs:21-191` - External app integration pattern
- `src-tauri/src/modules/file_copy.rs` - Async operations pattern
- `src/components/Settings.tsx` - Settings UI patterns

## User Configuration

**Settings stored in localStorage:**

```json
{
  "lightroomEnabled": true,
  "lightroomAutoMatch": true,
  "lightroomPresets": {
    "selects": "selects-jpeg-full",
    "delivery": "delivery-jpeg"
  }
}
```

**Plugin config (in LR):**

- Socket path: `~/.creatorops/lightroom.sock`
- Auto-start: Yes
- Permissions: Read catalog, write files

## Constraints

- Lightroom Classic must be running (acceptable per user)
- Manual plugin installation required (one-time)
- macOS primary (Windows/Linux: different socket paths)
- Requires Lightroom SDK 8.0+

## Sources

- [Creating collections with Lightroom Classic SDK](https://akrabat.com/creating-collections-with-the-lightroom-classic-sdk/)
- [Adobe Lightroom Classic Developer Portal](https://developer.adobe.com/lightroom-classic/)
- [Writing a Lightroom Classic plug-in](https://akrabat.com/writing-a-lightroom-classic-plug-in/)
- [Writing Lightroom Classic Plugins Guide](https://samrambles.com/guides/writing-lightroom-classic-plugins/index.html)
- [Jeffrey's "Run Any Command" Plugin](https://regex.info/blog/lightroom-goodies/run-any-command)
- [Lightroom SDK Examples on GitHub](https://github.com/Jaid/lightroom-sdk-8-examples)

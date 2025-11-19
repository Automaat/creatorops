# CreatorOps - Photography Workflow Management App

macOS desktop app for post-shoot photography workflow: SD card import, backup, archival, client delivery.

## Tech Stack

### Core
- **Tauri 2.0**: Rust backend + TypeScript/React frontend
- **Distribution**: DMG installer (drag-to-Applications)
- **Dependencies**: mise for toolchain management

### Key Libraries
- **nom-exif**: Photo/video metadata parsing (JPEG, RAW, MOV, MP4, etc.)
- **tokio**: Async file operations (4-8MB chunks optimized for ~1GB files)
- **tauri-plugin-fs-watch**: File system monitoring
- **React**: UI framework with TypeScript

## User Requirements

### File Handling
- Import: **copy** files from SD (preserve originals)
- File naming: keep original, optional rename in settings
- Delivery: full-res files for now
- Backup: manual trigger
- Multi-card: parallel import support
- Checksum failures: retry → skip with warning

### Storage
- External disk backup (now)
- Future: NAS, Google Drive
- Typical file size: ~1GB per file

### Integrations
- Lightroom Classic
- Aftershoot

### Usage
- Single user (no multi-user for now)
- Non-technical user installation (simple DMG)

## MVP Features

### 1. SD Card Import
- Auto-detect cards in `/Volumes/` directory
- Support parallel multi-card import
- Display card contents with file preview
- **Project selection**: import to existing project OR create new
- Copy files with progress tracking (speed, ETA, file count)
- MD5/SHA256 checksum verification
- Optional auto-rename based on settings
- Auto-eject option after successful import
- Duplicate detection

### 2. Project Management
- Create project: name, client, date, shoot type
- Select existing project on import
- Folder structure template: `YYYY-MM-DD_ClientName_ProjectType/[RAW, Selects, Delivery]`
- Project status pipeline: Importing → Editing → Delivered → Archived
- Dashboard: active projects, recent imports, storage stats
- Project detail view: metadata, stats, file browser, history

### 3. Backup System
- Manual trigger (no auto-backup for MVP)
- Configure multiple external disk destinations
- Copy entire project with checksum verification
- Retry on failure → skip with warning after retries
- Backup queue management (pending, in-progress, completed)
- Backup history and logs
- Extensible architecture for future NAS/cloud support

### 4. Archive Management
- Manual trigger from project view
- Move project to archive location
- Optional compression (zip/tar)
- Preserve folder structure
- Update project status to "Archived"
- Archive browser/search functionality

### 5. Client Delivery
- Select files from project for delivery
- Export to delivery folder
- Copy to external disk AND prep for upload
- Apply naming template (configurable)
- Generate file list manifest
- Future: direct Google Drive upload integration

### 6. Settings & Configuration
- Default import location
- Backup destinations (multiple paths)
- Archive location
- Folder naming templates
- File renaming rules (optional)
- Auto-actions (auto-eject, backup after import)
- File type filters

## UI/UX Flow

```
Main Window:
├─ Dashboard
│  ├─ Active projects overview
│  ├─ Recent imports
│  └─ Storage statistics
├─ Import
│  ├─ SD card detection/status
│  ├─ File browser/preview
│  └─ Project selection (new/existing)
├─ Projects
│  ├─ List view (search, filter by status)
│  └─ Project detail view
├─ Backup Queue
│  ├─ Pending backups
│  ├─ In-progress (with progress bars)
│  └─ Completed history
└─ Settings
   ├─ Paths configuration
   ├─ Templates
   ├─ Rename rules
   └─ Auto-actions

Project Detail View:
├─ Overview (metadata, stats, timeline)
├─ Files (browse folder structure)
├─ Actions (backup, deliver, archive)
└─ History (imports, backups, deliveries)
```

## Integration Points

### Lightroom Classic / Aftershoot
- Import files to monitored folders
- Catalog auto-detects new files
- Preserve folder structure for catalog compatibility

### External Disk
- Direct file copy via native file system
- No special protocols required

### Future: Google Drive
- OAuth authentication
- Official Google Drive API
- Upload delivery exports

### Future: NAS
- SMB/AFP protocol support
- Network path configuration

## Development Phases

### Phase 0: Project Setup ✓
1. Save detailed plan to `PLAN.md`
2. Initialize git repository
3. Create `.mise.toml` with Rust + Node.js toolchain
4. Initialize Tauri project (React + TypeScript)
5. Configure mise tasks: dev, build, lint, test, fmt, check
6. Setup Rust linting (clippy) + formatting (rustfmt)
7. Setup frontend linting (eslint) + formatting (prettier)
8. Setup test frameworks (cargo test + vitest)
9. Create `.gitignore`
10. Initial commit

### Phase 1: Foundation
- Tauri app scaffold with basic UI
- SD card detection (monitor `/Volumes/`)
- Basic file copy with progress tracking
- Project creation UI

### Phase 2: Core Workflow
- Project selection on import (new/existing)
- Folder structure templates
- Checksum verification (MD5/SHA256)
- Parallel multi-card import
- Import history tracking

### Phase 3: Backup System
- Backup to external disk
- Queue management
- Progress tracking per destination
- Checksum verification
- Retry logic with skip on failure
- Backup logs

### Phase 4: Delivery & Archive
- Client delivery export
- File selection UI
- Naming templates
- Archive functionality
- Compression options

### Phase 5: Settings & Polish
- Settings panel with all configuration
- Template editor
- Error handling and user notifications
- Keyboard shortcuts
- Import/backup history browser
- DMG packaging for distribution

## Future Enhancements

### Phase 6: Enhanced Import
- Photo/video thumbnail previews
- Smart duplicate detection (visual similarity)
- Import filtering (file type, date range)
- RAW + JPEG pairing detection
- Burst sequence detection

### Phase 7: Metadata & Organization
- Batch EXIF editing
- Keyword/tag system
- Search functionality
- Storage analytics (disk usage by project)

### Phase 8: Cloud & Network
- Google Drive upload integration
- Dropbox support
- NAS/network storage support
- Multi-location sync

### Phase 9: Advanced Features
- AI culling assistance
- Automated proof gallery generation
- Time-lapse sequence detection
- Scheduled archival rules
- Webhook notifications
- Mobile companion app (iOS)

### Phase 10: Professional Features
- Integration with Lightroom/Capture One catalogs
- Version control for edited files
- Project timeline visualization
- Print order preparation
- Contract/invoice integration
- Client portal integration

## Industry Best Practices

### Professional Workflow Patterns
- **3-2-1 backup rule**: 3 copies, 2 different media, 1 offsite
- **Dual card shooting**: Backup starts in-camera
- **Fast SSD staging**: Import to fast drive, then archive to NAS
- **Automation critical**: Complex workflows get skipped
- **Early metadata**: Add IPTC/keywords during import
- **Non-destructive edits**: Keep RAW originals untouched

### Common Folder Structures
- Date-based: `YYYY/MM/DD-EventName/`
- Client-based: `ClientName/YYYY-MM-DD-ProjectType/`
- Project-based: `YYYY-MM-DD_ClientName_Type/[RAW, Selects, Delivery]` ← **Our default**

## Technical Architecture

### File Copy Engine
- Async operations with tokio
- 4-8MB chunk size (optimized for ~1GB files)
- Progress events to frontend (speed, ETA, percentage)
- MD5/SHA256 checksum calculation
- Parallel copy for multiple sources
- Error recovery: retry → skip with warning

### SD Card Detection
- Monitor `/Volumes/` directory for new mounts
- Detect card type (SD, CF, etc.) via volume info
- Auto-detect photo/video file patterns
- Support multiple simultaneous cards

### Metadata Parsing
- Use nom-exif crate (pure Rust)
- Support formats: JPEG, HEIF/HEIC, TIFF, RAW, MOV, MP4, etc.
- Extract: date, camera model, settings, GPS
- Use for auto-naming and organization

### Progress Tracking
- Tauri event system for real-time updates
- Track per-file and overall progress
- Calculate speed (MB/s) and ETA
- Update UI without blocking operations

### Error Handling
- Checksum mismatch: retry up to 3 times → skip with warning
- Disk full: pause and notify user
- Card disconnected: pause and allow resume
- Permission errors: clear error messages

## Research Summary

### macOS App Development (2025)
- **Tauri 2.0** preferred over Electron:
  - 95% smaller app size (~3-25MB vs 50MB-1.3GB)
  - Better performance
  - Native macOS integration
  - Built-in file system, notifications, auto-updates
  - Limitation: No security-scoped bookmarks (not needed outside App Store)

### Photography Workflow Tools
- **Popular software**: Lightroom, Capture One, Affinity Photo, ACDSee
- **Trends**: AI integration, Apple Silicon optimization, non-destructive editing
- **Automation tools**: ChronoSync, GoodSync, Carbon Copy Cloner, Automator
- **Cloud backup**: Backblaze ($7/month unlimited)

### Metadata Libraries
- **nom-exif**: Best choice - supports photos + videos in pure Rust
- **kamadak-exif**: Photos only, pure Rust
- **rexiv2**: Wraps C++ library (avoid)

### File Watching
- **tauri-plugin-fs-watch**: Official Tauri plugin
- **notify crate**: Underlying Rust library with debouncing

## Open Questions & Decisions Made

### Resolved
1. ✓ Import behavior: **copy** files (not move)
2. ✓ File naming: **keep original**, optional rename in settings
3. ✓ Delivery size: **full-res only** for MVP
4. ✓ Backup trigger: **manual** for MVP
5. ✓ Multiple cards: **parallel import**
6. ✓ Checksum failures: **retry → skip with warning**
7. ✓ Project on import: **select existing OR create new**
8. ✓ Distribution: **DMG** (not App Store)
9. ✓ File size typical: **~1GB**
10. ✓ Backup destinations: **external disk** (NAS/cloud future)
11. ✓ Catalog integration: **Lightroom Classic, Aftershoot**
12. ✓ Delivery method: **folder + external disk** (cloud future)
13. ✓ Archive frequency: **manual trigger**
14. ✓ Multi-user: **no** (single user for now)
15. ✓ File types: **all files** (mostly photo/video)

### Potential Future Considerations
- Auto-backup after import (toggle in settings)
- Smart project suggestion based on date/metadata
- Cloud backup destinations (Google Drive, Dropbox)
- Network/NAS support
- Mobile companion app
- Batch operations UI
- Project templates library
- Client portal integration

---

**Last Updated**: 2025-11-19
**Status**: Phase 4 complete

## Implementation Status

### Phase 0: Project Setup ✓
- Git repository initialized
- Tauri 2.0 + React + TypeScript configured
- mise toolchain management setup
- Linting, formatting, testing configured
- Initial commit complete

### Phase 1: Foundation ✓
- Tauri app with basic UI and theme system
- SD card detection from `/Volumes/`
- File copy with progress tracking
- Project creation UI and folder structure
- Basic navigation and layout

### Phase 2: Core Workflow ✓
- **Project selection on import**: Choose existing project or create new
- **Folder structure templates**: `YYYY-MM-DD_ClientName_Type/[RAW, Selects, Delivery]`
- **Checksum verification**: SHA-256 with 3-retry logic, skip on failure
- **Parallel multi-card import**: Each card can be imported independently
- **Import history tracking**: All imports saved with metadata and status
- **Progress tracking**: Real-time updates with speed, ETA, file counts
- **File listing**: Photo/video file detection (JPEG, RAW, MOV, MP4, etc.)

### Phase 3: Backup System ✓
- **Backup queue management**: Queue, start, cancel, remove backup jobs
- **Backup to external disk**: Copy entire projects with folder structure preservation
- **Multiple destinations**: Configure and manage multiple backup destinations in Settings
- **Checksum verification**: SHA-256 with 3-retry logic per file
- **Progress tracking**: Real-time updates with speed, ETA, file counts per backup job
- **Retry logic**: Exponential backoff with jitter, skip files after max retries
- **Backup history**: Track all backup operations with status and metadata
- **UI components**: BackupQueue view, Projects list/detail view with backup actions
- **Settings integration**: Add/remove/enable backup destinations with folder picker

### Phase 4: Delivery & Archive ✓

- **Client delivery system**: Select files from projects for client delivery
- **File selection UI**: Browse project files, select individual files or select all
- **Naming templates**: Apply templates with {index}, {name}, {ext} placeholders
- **Delivery destinations**: Configure multiple delivery destinations in Settings
- **Manifest generation**: Auto-generate delivery manifest with file mapping
- **Delivery queue**: Track delivery jobs with real-time progress
- **Archive functionality**: Move projects to archive location with structure preservation
- **Archive location config**: Configure archive location in Settings
- **Project status update**: Automatically update project status to "Archived"
- **UI components**: Delivery view with multi-step workflow, archive action in Projects view
- **Progress tracking**: Real-time updates for both delivery and archive operations

# Clients View Implementation Plan

## Overview
Add separate Clients view for managing client information, with one-to-many relationship to Projects. Minimal but useful Phase 1 implementation following existing app patterns.

## Architecture Approach

### Hybrid Denormalization Strategy
- Keep both `client_id` (FK) and `client_name` (denormalized) in projects table
- Avoids JOIN in hot paths (list_projects), maintains existing UI compatibility
- Sync both fields on updates

### Data Model
```typescript
Client {
  id: string (UUID)
  name: string (UNIQUE)
  email?: string
  phone?: string
  notes?: string
  createdAt: string
  updatedAt: string
}

Project {
  // existing fields...
  clientId?: string (FK to clients, nullable during migration)
  clientName: string (denormalized, kept in sync)
}
```

## Implementation Steps

### 1. Database & Backend (Rust)

**File: `src-tauri/src/modules/db.rs`**
- Add clients table with UNIQUE constraint on name
- Add `client_id` FK column to projects (nullable)
- Create indexes: `idx_clients_name`, `idx_projects_client_id`

**File: `src-tauri/src/modules/client.rs` (new)**
CRUD commands:
- `create_client(name, email?, phone?, notes?)` - validate, handle UNIQUE constraint
- `list_clients()` - return sorted alphabetically
- `get_client(client_id)` - return client + all projects
- `update_client(...)` - sync projects.client_name when name changes
- `delete_client(client_id)` - prevent if has projects
- `search_clients(query)` - search by name/email
- `migrate_clients_from_projects()` - one-time migration

**File: `src-tauri/src/lib.rs`**
- Import and register all client commands

**File: `src-tauri/src/modules/project.rs`**
- Update `create_project` to accept `client_id` instead of `client_name`
- Fetch client name from clients table for folder naming
- Insert both `client_id` and `client_name`

### 2. Frontend Types

**File: `src/types/index.ts`**
- Add `Client` interface
- Add `ClientWithProjects` interface
- Update `Project` interface with `clientId?: string`

### 3. Core Components

**File: `src/components/Clients.tsx` (new)**
Dual-mode component following Projects.tsx pattern:
- **List mode**: Grid of client cards, search bar, "Create Client" button
- **Detail mode**: Client metadata + projects list + delete button
- State: clients, selectedClient, searchQuery, showCreateClient, showDeleteDialog
- ESC key navigation back to list

**File: `src/components/CreateClient.tsx` (new)**
Form with fields: name (required), email, phone, notes
- Follow CreateProject.tsx styling
- Handle UNIQUE constraint errors

**File: `src/components/ClientSelector.tsx` (new)**
Searchable dropdown for CreateProject form:
- Load and filter clients by search
- Show "Create new client" option
- Inline client creation flow

### 4. Integration

**File: `src/components/CreateProject.tsx`**
- Replace `clientName` text input with `ClientSelector`
- Add inline client creation state
- Update formData to use `clientId`

**File: `src/App.tsx`**
- Add 'clients' to View type
- Add selectedClientId state
- Add Cmd+4 keyboard shortcut
- Add clients view div with display toggling
- Pass clientsCount to Layout

**File: `src/components/Layout.tsx`**
- Add Clients nav item to Main section
- Add clients icon assets (30x30px PNG)

### 5. Migration

**File: `src/components/Dashboard.tsx`**
Add migration trigger in useEffect:
- Check localStorage flag: `clients_migrated`
- If not set, invoke `migrate_clients_from_projects()`
- Handle duplicate names (append suffix: "Name (2)")
- Set flag on completion

### 6. Testing

**Rust tests (`src-tauri/src/modules/client.rs`):**
- UNIQUE constraint validation
- Delete protection with projects
- Search case-insensitive
- Update syncs project names

**React tests:**
- `Clients.test.tsx` - list/detail/search/empty states
- `CreateClient.test.tsx` - validation, creation, error handling
- `ClientSelector.test.tsx` - load/filter/select/create

**Integration:**
- Full workflow: create client → create project → verify in client detail
- Update client name → verify project synced
- Delete protection → delete projects → delete client

## UI/UX Following Design System

### List View
- Search bar: 40px height, full width, debounced 300ms
- Grid: `repeat(auto-fill, minmax(300px, 1fr))`, 16px gap
- Client cards: white bg, 24px padding, shadow-card, hover shadow-card-hover
- Card content: name (title), email, phone, project count

### Detail View
- Header: Back button + name + Edit button
- Metadata: Email, phone (if set)
- Notes: Text block with 16px padding
- Projects section: Grid of project cards (reuse existing styles)
- Delete button: btn-danger style, bottom of view

### Empty States
- No clients: "No clients yet" + "Click Create Client to get started"
- No projects for client: "No projects for this client yet" + Create Project button

### Typography & Spacing
- Follow 8px grid (spacing multiples of 8)
- Use semantic type tokens (title/section/body/meta)
- Generous whitespace (section padding 40px)

## Critical Files

### New Files
- `src-tauri/src/modules/client.rs` - Client CRUD logic
- `src/components/Clients.tsx` - Main view (list + detail)
- `src/components/CreateClient.tsx` - Form modal
- `src/components/ClientSelector.tsx` - Searchable dropdown
- `src/components/Clients.test.tsx` - Component tests
- `src/components/CreateClient.test.tsx` - Form tests
- `src/components/ClientSelector.test.tsx` - Selector tests

### Modified Files
- `src-tauri/src/modules/db.rs` - Schema changes
- `src-tauri/src/modules/project.rs` - Update create_project
- `src-tauri/src/lib.rs` - Register commands
- `src/types/index.ts` - Client types
- `src/components/CreateProject.tsx` - Use ClientSelector
- `src/App.tsx` - Add clients view routing
- `src/components/Layout.tsx` - Add nav item
- `src/components/Dashboard.tsx` - Migration trigger

## Implementation Sequence

1. **Database & Backend** - client.rs module, schema, CRUD commands, tests
2. **Frontend Core** - Types, Clients.tsx list view, CreateClient.tsx
3. **Projects Integration** - ClientSelector, update CreateProject
4. **Migration & Polish** - Migration trigger, search, delete flow, styling
5. **Testing** - Unit tests, component tests, E2E validation

## Design Decisions

### Hybrid Denormalization
Keep both client_id and client_name in projects for performance (avoid JOINs in list_projects). Acceptable trade-off: 2x storage for small strings.

### Nullable FK During Migration
client_id nullable ensures backward compatibility during migration. App remains functional if migration fails.

### Delete Protection
Prevent deleting clients with projects. User must delete projects first. Maintains data integrity, clear UX.

### Simple Phase 1
No tags, categories, or advanced features. Name/email search only. Add complexity only if needed (YAGNI).

### Searchable Dropdown
Client selector shows all clients (discourages duplicates), supports inline creation (no context switch).

### User Decisions
1. **Email validation**: Validate email format (regex: `^[^\s@]+@[^\s@]+\.[^\s@]+$`)
2. **Phone storage**: Plain text, no formatting/validation
3. **Keyboard shortcut**: Cmd+4 for Clients (shift others: backup→5, delivery→6, history→7, settings→8)
4. **Client archival**: Add status field ('active'|'archived'), soft delete
5. **Duplicate merge**: Manual cleanup only, no merge UI

## Updated Data Model

```typescript
Client {
  id: string (UUID)
  name: string (UNIQUE)
  email?: string (validated)
  phone?: string (plain text)
  notes?: string
  status: 'active' | 'archived'  // NEW
  createdAt: string
  updatedAt: string
}
```

## Updated Commands

**Backend:**
- `list_clients(include_archived: bool)` - filter by status
- `update_client_status(client_id, status)` - archive/unarchive
- Email validation in create_client and update_client

**Frontend:**
- Clients.tsx: Add filter toggle (Active/Archived/All), archive button
- CreateClient.tsx: Email input validation with regex

## Actionable Implementation Checklist

### Phase 1: Backend (90min)
- [ ] `src-tauri/src/modules/db.rs`: Add clients table with status column, projects.client_id FK, indexes
- [ ] `src-tauri/src/modules/client.rs`: Create module with 8 commands + email validation + tests
- [ ] `src-tauri/src/modules/mod.rs`: Add client module
- [ ] `src-tauri/src/lib.rs`: Register all client commands
- [ ] `src-tauri/src/modules/project.rs`: Update create_project to accept client_id
- [ ] Verify: `cargo test && cargo build`

### Phase 2: Frontend Types & Forms (45min)
- [ ] `src/types/index.ts`: Add Client, ClientWithProjects interfaces, update Project
- [ ] `src/components/CreateClient.tsx`: Form with email validation + test
- [ ] Verify: `npm run build && npm test`

### Phase 3: Clients View (105min)
- [ ] `src/components/Clients.tsx`: List mode (search, filter, grid)
- [ ] `src/components/Clients.tsx`: Detail mode (metadata, projects, archive, delete)
- [ ] `src/components/Clients.test.tsx`: Component tests
- [ ] Verify: Component renders, search/filter work

### Phase 4: Integration (80min)
- [ ] `src/components/ClientSelector.tsx`: Searchable dropdown + inline create + test
- [ ] `src/components/CreateProject.tsx`: Replace clientName with ClientSelector
- [ ] `src/App.tsx`: Add clients view, Cmd+4 shortcut, update other shortcuts
- [ ] `src/components/Layout.tsx`: Add Clients NavItem + icons
- [ ] `src/components/Dashboard.tsx`: Add migration trigger
- [ ] Verify: Full create client → create project flow works

### Phase 5: Polish & Testing (65min)
- [ ] Add empty states
- [ ] Verify 8px grid spacing, semantic colors
- [ ] E2E test: create, update, archive, delete flows
- [ ] Test migration with existing data
- [ ] Verify: `npm run format:check lint test -- --run build`
- [ ] Verify: `mise run fmt lint test`

**Total: ~6 hours**

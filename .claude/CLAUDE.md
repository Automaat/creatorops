# CreatorOps

Photography workflow desktop app: import, backup, deliver, archive.

**Stack**: React 19 + TypeScript 5.8 + Vite 7 (frontend) | Rust 1.91 + Tauri 2.9 + SQLite (backend) | Vanilla CSS (NO Tailwind/CSS-in-JS) | Vitest + cargo test | mise tooling

## Setup & Development

```bash
mise install && npm install  # Setup
mise run dev                 # Start dev mode
mise run check               # Verify all (fmt + lint + test)
```

## Project Structure

```text
src/              # React frontend (components/, hooks/, styles/, utils/, types/)
src-tauri/src/    # Rust backend (lib.rs, modules/*.rs)
.mise.toml        # Tool versions & tasks
```

## Commands

| Task | Command |
|------|---------|
| Dev | `mise run dev` |
| Test | `npm run test -- --run` (frontend), `mise run test:rust` (backend) |
| Lint | `npm run lint` (ESLint), `mise run lint:rust` (clippy) |
| Format | `npm run format` (Prettier), `cargo fmt` |
| Build | `npm run build`, `mise run build` (Tauri .dmg) |
| **Verify all** | `mise run check` |

## TypeScript/React Patterns

### Components

```typescript
// Functional only, props interface, co-located tests
interface DashboardProps {
  onProjectClick?: (projectId: string) => void
}

export function Dashboard({ onProjectClick }: DashboardProps) {
  const [projects, setProjects] = useState<Project[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => { loadData() }, [])

  async function loadData() {
    try {
      setLoading(true)
      const projectList = await invoke<Project[]>('list_projects')
      setProjects(projectList)
    } catch (err) {
      console.error('Failed to load projects:', err)
    } finally {
      setLoading(false)
    }
  }
}
```

**Naming**: Components `PascalCase`, props `ComponentNameProps`, callbacks `on*`, state setters `set*`

### State

- Local: `useState` for UI state
- Context: Only cross-cutting (NotificationContext)
- NO Redux/Zustand
- Data: Tauri `invoke()` with try/catch/finally

### Hooks

```typescript
// Prefix use*, return objects, cleanup in useEffect
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(() =>
    (localStorage.getItem('theme') as Theme) || 'system'
  )

  useEffect(() => {
    // Apply theme + persist
    localStorage.setItem('theme', theme)
  }, [theme])

  return { theme, setTheme }
}
```

### Styling (CRITICAL)

**Vanilla CSS only. NO Tailwind, CSS-in-JS, inline styles.**

```css
/* Use design tokens from variables.css */
.form-footer {
  margin-top: var(--space-lg);      /* 8px grid: 8,16,24,32,40,48 */
  padding: var(--space-md);
  background: var(--color-bg-primary);
  box-shadow: var(--shadow-card);
}
```

**Bad**: `<div style={{ marginTop: '24px' }}>`
**Good**: `<div className="form-footer">`

### Types

```typescript
// src/types/index.ts - single file
export enum ProjectStatus { New = 'New', Editing = 'Editing', ... }
export interface Project { id: string; name: string; status: ProjectStatus; deadline?: string; ... }
export type BackupStatus = 'pending' | 'inprogress' | 'completed' | 'failed' | 'cancelled'
```

### Error Handling (Frontend)

```typescript
const { success, error } = useNotification()  // NOT alert()
try {
  const result = await invoke<Project>('create_project', formData)
  success('Project created')
} catch (err) {
  console.error('Failed:', err)
  error('Failed to create project')
} finally {
  setLoading(false)
}
```

### Imports

```typescript
// Grouped: React → Tauri → Types → Utils → Components → Hooks
import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'
import { sortProjects } from '../utils/project'
import { CreateProject } from './CreateProject'
import { useSDCardScanner } from '../hooks/useSDCardScanner'
```

## Rust Patterns

### Module Structure

```rust
// Flat modules in src-tauri/src/modules/, snake_case naming
pub mod backup;  // backup.rs
pub mod db;      // db.rs
```

### Error Handling (Backend)

```rust
// Result<T, String> for Tauri commands, use log crate (not eprintln!)
use log::{info, error};

pub fn with_db<F, R>(f: F) -> Result<R, String>
where F: FnOnce(&Connection) -> Result<R, String>
{
    let db = DB_CONNECTION.lock().map_err(|e| format!("Lock failed: {}", e))?;
    let conn = db.as_ref().ok_or_else(|| "DB not initialized".to_string())?;
    f(conn)
}
```

### Async & Tauri

```rust
#[tauri::command]
pub async fn start_backup(window: tauri::Window, job_id: String) -> Result<BackupJob, String> {
    let job = update_job_status(&job_id, BackupStatus::InProgress)?;

    // Spawn background task, return immediately
    tokio::spawn(async move {
        if let Err(e) = perform_backup(&window, &job_id, &job).await {
            error!("Backup failed: {}", e);
        }
    });
    Ok(job)
}

// Serialization with camelCase for TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupJob {
    pub id: String,
    pub project_id: String,  // → projectId in JSON
    pub status: BackupStatus,
}

// Emit events
let _ = window.emit("backup-progress", progress);
```

### Documentation

```rust
/// Opens project media folder in external app.
///
/// Assumes structure: ProjectFolder/RAW/Photos/ or RAW/Videos/
/// Launches app in background (fire-and-forget).
pub fn open_in_external_app(path: &str, subfolder: &str, app: &str) -> Result<(), String>
```

## Testing

### Frontend (Vitest + RTL)

```typescript
import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

describe('Dashboard', () => {
  it('loads and displays projects', async () => {
    vi.mocked(invoke).mockResolvedValue(mockProjects)
    render(<Dashboard />)
    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())
  })
})
```

### Backend (Cargo)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_backup_status_serialization() {
        assert_eq!(serde_json::to_string(&BackupStatus::Pending).unwrap(), r#""pending""#);
    }
}
```

### Bug Fix Workflow

1. Write failing test
2. Fix root cause
3. Verify test passes

## Linting & Formatting

| Frontend | Backend |
|----------|---------|
| Prettier: single quotes, no semicolons, width 100 | rustfmt: edition 2021, max_width 100 |
| ESLint: max-warnings: 0 | clippy: -D warnings |
| **NEVER** `eslint-disable`, `@ts-ignore` | **NEVER** `#[allow(clippy::*)]` except `too_many_arguments` |

## Git Conventions

**Commits** (conventional, signed):

```bash
git commit -s -S -m "feat: add backup queue"
git commit -s -S -m "fix: checksum verification"
git commit -s -S -m "refactor: extract project sorting"
```

**Types**: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`

**PRs**:

```markdown
## Motivation
[Why]

## Implementation information
[How, alternatives]

## Supporting documentation
[Issues, PRs, docs]
```

## Anti-Patterns (AVOID)

1. Inline styles
2. Linter disables (`eslint-disable`, `@ts-ignore`, `#[allow]`)
3. Magic numbers (use 8px grid)
4. Over-engineering
5. Multiple accent colors (stick to orange `#D68406`)
6. Generic errors

## Good Patterns (FOLLOW)

1. Spacious minimalism (8px grid)
2. Design tokens (CSS variables)
3. Try/catch/finally
4. Event-driven progress (Tauri emitters)
5. Type safety (strict TS, Rust)
6. Co-located tests

## Code Generation Rules

**Adding features**: Read existing → Follow conventions → Write tests → Use tokens → Document APIs → Verify (`mise run check`)

**Fixing bugs**: Failing test → Fix root cause → Verify → Add regression test

**Refactoring**: Maintain tests → Minimal scope → Extract only if 3+ uses

## API Integration

OAuth tokens in system keyring, never commit credentials.

```rust
use keyring::Entry;

pub async fn get_access_token() -> Result<String, String> {
    Entry::new("CreatorOps", "google_drive_token")?.get_password().map_err(|e| e.to_string())
}
```

## Data Persistence

- **Projects/deliveries**: SQLite (`~/CreatorOps/creatorops.db`)
- **Backup/import history**: JSON files (`~/CreatorOps/backup_history.json`)
- **Settings**: localStorage (frontend)

## Performance

**Frontend**: `useMemo` for expensive calcs, `useCallback` for callbacks, code-split if grows

**Backend**: Semaphore (4 max concurrent), 4MB chunks, `tokio::spawn` for long ops, CancellationToken

## Accessibility

Semantic HTML (`<button>` not `<div onClick>`), `:focus-visible`, `.visually-hidden`, WCAG AA contrast, keyboard shortcuts

## Known Technical Debt

1. Frontend: `alert()` → NotificationContext
2. Backend: `Result<T, String>` → custom errors with `thiserror`
3. Backend: `eprintln!` → `log` crate
4. No React error boundaries
5. SQLite: no transactions
6. `Settings.tsx:30`: fix `eslint-disable`
7. `Projects.tsx:124-137`: extract inline styles

## Questions for AI

1. Use CSS variables from variables.css?
2. Local state or context?
3. Test cases to cover?
4. NotificationContext or throw?
5. Need useMemo/useCallback?
6. Existing pattern for this?

## Success Checklist

- [ ] Tests pass (`mise run test`)
- [ ] Lint passes (`mise run lint`)
- [ ] Format passes (`npm run format:check`, `cargo fmt --check`)
- [ ] Build succeeds
- [ ] No linter disables added
- [ ] No inline styles
- [ ] Design tokens used
- [ ] Tests written
- [ ] Conventional commit
- [ ] Docs updated (if public API changed)

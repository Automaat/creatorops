# Code Review Instructions

## Project Context

**Stack:**
- Frontend: React 19, TypeScript 5.8, Vite 7
- Backend: Rust 1.91 (edition 2021), Tauri 2.9
- Styling: Vanilla CSS ONLY (NO Tailwind, NO CSS-in-JS, NO inline styles)
- State: useState + Context (NO Redux/Zustand)
- Testing: Vitest (frontend), cargo test (backend)
- Tools: mise, ESLint, Prettier, Clippy

**Purpose:**
Photography workflow desktop app - import from SD cards, backup, archival, client delivery. macOS DMG distribution.

**Core Modules:**

Frontend (`src/`):
- `components/`: React functional components (PascalCase, co-located tests)
- `hooks/`: Custom hooks (prefix `use*`, return objects)
- `contexts/`: React Context (NotificationContext, ThemeContext)
- `styles/`: Vanilla CSS with design tokens (variables.css)
- `utils/`: Pure functions, helpers
- `types/`: TypeScript interfaces

Backend (`src-tauri/src/`):
- `lib.rs`: Tauri commands, app setup
- `modules/`: Domain modules (import, backup, project, etc.)

**Conventions:**

Frontend:
- Components: Functional only, props interface (`ComponentNameProps`), callbacks `on*`
- State: Local (`useState`), Context for cross-cutting only
- Error Handling: try/catch/finally, user-facing errors via notifications
- Styling: **CRITICAL** - Vanilla CSS only, design tokens from `variables.css`
- Naming: Components `PascalCase`, files match component, props interfaces, state setters `set*`

Backend:
- Error Handling: `anyhow::Result` in commands, `thiserror` in library code
- Async: tokio runtime, async for file I/O
- No unwrap/expect/panic (Clippy denies)
- Tauri commands: `#[tauri::command]`, return `Result<T, String>`

**Critical Areas (Extra Scrutiny):**
- Vanilla CSS compliance (NO Tailwind classes, NO inline styles)
- File I/O (SD card import, backup, checksum verification)
- Tauri command boundaries (frontend ↔ backend)
- Error handling (user-facing messages)
- Performance (large file handling ~1GB per file)

---

## Review Before CI Completes

You review PRs immediately, before CI finishes. Do NOT flag issues that CI will catch.

**CI Already Checks:**

Frontend:
- Formatting (Prettier)
- Linting (ESLint - TypeScript recommended + react-hooks)
- Type checking (tsc)
- Tests (Vitest)
- Build (Vite)

Backend:
- Formatting (cargo fmt)
- Linting (Clippy - all + pedantic + nursery, see Cargo.toml)
- Tests (cargo test)
- Build (cargo build)

---

## Review Priority Levels

### 🔴 CRITICAL (Must Block PR)

**Styling Violations** (95%+ confidence)
- [ ] Tailwind classes used (className="flex items-center")
- [ ] CSS-in-JS (styled-components, emotion)
- [ ] Inline styles (style={{ color: 'red' }})
- [ ] Must use vanilla CSS with design tokens

**Correctness Issues** (90%+ confidence)
- [ ] File operations without error handling (data loss risk)
- [ ] Checksum verification skipped
- [ ] SD card unmount before import complete
- [ ] Tauri command panics (violates Clippy deny)
- [ ] `.unwrap()` or `.expect()` in Rust production code
- [ ] React hooks rules violated (conditional hooks, missing deps)

**Security Issues** (85%+ confidence)
- [ ] Path traversal vulnerabilities
- [ ] User input not validated (file paths, project names)
- [ ] Credentials stored insecurely
- [ ] File permissions not checked

### 🟡 HIGH (Request Changes)

**Architecture Violations** (80%+ confidence)
- [ ] Redux/Zustand introduced (use useState + Context)
- [ ] Global state outside Context
- [ ] Class components (must be functional)
- [ ] Prop drilling >3 levels (use Context)

**Error Handling** (85%+ confidence)
- [ ] Errors not shown to user (need notification)
- [ ] Generic error messages ("failed" vs "SD card disconnected")
- [ ] Missing try/catch on Tauri invoke
- [ ] Rust panics not converted to Result
- [ ] anyhow in Rust library code (use thiserror)

**Testing** (80%+ confidence)
- [ ] New components without tests
- [ ] Tauri commands without tests
- [ ] File operations untested
- [ ] Critical paths (import, backup) untested

**Type Safety** (75%+ confidence)
- [ ] `any` type used
- [ ] Type assertions without validation (`as Type`)
- [ ] Missing interfaces for props
- [ ] Implicit return types

### 🟢 MEDIUM (Suggest/Comment)

**Performance** (70%+ confidence)
- [ ] Large file operations not chunked (>4MB)
- [ ] Synchronous file I/O in async context
- [ ] Unnecessary re-renders (missing memo/callback)
- [ ] N+1 queries to backend

**Code Quality** (65%+ confidence)
- [ ] Components >200 lines
- [ ] Functions >50 lines
- [ ] Nested conditionals (use early returns)
- [ ] Magic strings/numbers (use constants)
- [ ] Duplicated logic

**Documentation** (60%+ confidence)
- [ ] Complex logic without comments
- [ ] Public Tauri commands without docs
- [ ] Hooks without usage examples

### ⚪ LOW (Optional/Skip)

Don't comment on:
- Formatting (Prettier/rustfmt handles)
- ESLint/Clippy warnings (CI handles)
- Import order
- Style preferences

---

## Styling Guidelines (CRITICAL)

### ❌ NEVER Use

**Tailwind classes:**
```tsx
// WRONG - Tailwind
<div className="flex items-center justify-between p-4 bg-blue-500">
```

**CSS-in-JS:**
```tsx
// WRONG - styled-components
const Button = styled.button`
  background: blue;
`;
```

**Inline styles:**
```tsx
// WRONG - inline styles
<div style={{ display: 'flex', padding: '16px' }}>
```

### ✅ ALWAYS Use

**Vanilla CSS with design tokens:**
```tsx
// RIGHT - vanilla CSS
<div className="card-header">

// card.css
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-md);
  background: var(--color-bg-secondary);
}
```

**Design tokens from variables.css:**
- Spacing: `var(--space-xs)` through `var(--space-xl)` (8px grid: 8, 16, 24, 32, 40, 48)
- Colors: `var(--color-bg-primary)`, `var(--color-text-primary)`, etc.
- Borders: `var(--border-radius-sm)`, `var(--border-width)`

---

## Frontend Guidelines (React + TypeScript)

### Component Structure

```tsx
// ComponentName.tsx
interface ComponentNameProps {
  onAction?: (id: string) => void
  disabled?: boolean
}

export function ComponentName({ onAction, disabled }: ComponentNameProps) {
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    // Side effects
    return () => {
      // Cleanup
    }
  }, [])

  async function handleClick() {
    try {
      setLoading(true)
      await invoke('tauri_command')
      onAction?.(id)
    } catch (err) {
      console.error('Action failed:', err)
      // Show notification
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="component-name">
      {/* JSX */}
    </div>
  )
}
```

### Naming Conventions

- **Components:** `PascalCase` (Dashboard, ImportCard)
- **Props interface:** `ComponentNameProps`
- **Files:** Match component name (`Dashboard.tsx`)
- **Callbacks:** `on*` prefix (onClick, onSubmit)
- **State setters:** `set*` prefix (setLoading, setProjects)
- **CSS classes:** `kebab-case` (card-header, button-primary)

### State Management

**Local state:**
```tsx
const [projects, setProjects] = useState<Project[]>([])
```

**Context (cross-cutting only):**
```tsx
const { showNotification } = useNotificationContext()
```

**NO Redux, NO Zustand** - use useState + Context

### Tauri Invoke Pattern

```tsx
async function loadData() {
  try {
    setLoading(true)
    const data = await invoke<Project[]>('list_projects')
    setProjects(data)
  } catch (err) {
    console.error('Failed to load:', err)
    showNotification('Failed to load projects', 'error')
  } finally {
    setLoading(false)
  }
}
```

### Custom Hooks

```tsx
// useHookName.ts
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(() =>
    (localStorage.getItem('theme') as Theme) || 'system'
  )

  useEffect(() => {
    localStorage.setItem('theme', theme)
    applyTheme(theme)
  }, [theme])

  return { theme, setTheme }  // Return object, not array
}
```

---

## Backend Guidelines (Rust + Tauri)

### Tauri Commands

```rust
#[tauri::command]
async fn import_files(
    source: String,
    destination: String,
) -> Result<ImportResult, String> {
    import_files_internal(&source, &destination)
        .await
        .map_err(|e| format!("Import failed: {}", e))
}

// Internal with proper error types
async fn import_files_internal(
    source: &str,
    destination: &str,
) -> anyhow::Result<ImportResult> {
    validate_paths(source, destination)?;
    let files = discover_files(source).await?;
    copy_with_verification(files, destination).await?;
    Ok(ImportResult { /* ... */ })
}
```

### Error Handling (Strict - No Unwrap)

**Tauri commands:** Return `Result<T, String>` for frontend
**Internal code:** Use `anyhow::Result` with `.context()`
**Library modules:** Use `thiserror` for custom errors
**NEVER:** `.unwrap()`, `.expect()`, `panic!()` (Clippy denies)

```rust
// Library module
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("SD card not found: {0}")]
    CardNotFound(String),
    #[error("checksum mismatch: {path}")]
    ChecksumFailed { path: String },
}

// App code
fn validate_checksum(path: &Path, expected: &str) -> anyhow::Result<()> {
    let actual = compute_sha256(path)
        .context("failed to compute checksum")?;

    if actual != expected {
        anyhow::bail!("checksum mismatch: {}", path.display());
    }
    Ok(())
}
```

### Async File Operations

```rust
use tokio::fs;
use tokio::io::AsyncReadExt;

async fn copy_file_chunked(src: &Path, dst: &Path) -> anyhow::Result<()> {
    const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB chunks for ~1GB files

    let mut src_file = fs::File::open(src).await?;
    let mut dst_file = fs::File::create(dst).await?;

    let mut buffer = vec![0; CHUNK_SIZE];
    loop {
        let n = src_file.read(&mut buffer).await?;
        if n == 0 { break; }
        dst_file.write_all(&buffer[..n]).await?;
    }
    Ok(())
}
```

### Clippy Compliance (Strict)

Project uses:
- `all = "deny"` (700+ lints)
- `pedantic = "warn"`
- `nursery = "warn"`
- `unwrap_used = "deny"`
- `expect_used = "deny"`
- `panic = "deny"`

Exception:
- `wildcard_imports = "allow"` (Tauri macros need this)

---

## Testing Requirements

### Frontend Tests (Vitest + React Testing Library)

```tsx
// ComponentName.test.tsx
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ComponentName } from './ComponentName'

describe('ComponentName', () => {
  it('calls onAction when clicked', async () => {
    const onAction = vi.fn()
    render(<ComponentName onAction={onAction} />)

    await userEvent.click(screen.getByRole('button'))

    await waitFor(() => {
      expect(onAction).toHaveBeenCalledWith('expected-id')
    })
  })
})
```

**Required tests:**
- [ ] Components render correctly
- [ ] User interactions work
- [ ] Loading states shown
- [ ] Errors handled gracefully

### Backend Tests (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_import_files() {
        let temp = tempdir().unwrap();
        let src = temp.path().join("source");
        let dst = temp.path().join("dest");

        // Setup
        fs::create_dir(&src).await.unwrap();
        fs::write(src.join("test.jpg"), b"data").await.unwrap();

        // Execute
        let result = import_files_internal(
            src.to_str().unwrap(),
            dst.to_str().unwrap()
        ).await;

        // Verify
        assert!(result.is_ok());
        assert!(dst.join("test.jpg").exists());
    }
}
```

**Required tests:**
- [ ] Tauri commands work
- [ ] File operations succeed
- [ ] Error cases handled
- [ ] Edge cases covered

---

## Review Examples

### ✅ Good: Vanilla CSS
```tsx
// Component
<div className="import-card">
  <button className="button-primary">Import</button>
</div>

// import-card.css
.import-card {
  padding: var(--space-md);
  background: var(--color-bg-secondary);
  border-radius: var(--border-radius-md);
}

.button-primary {
  background: var(--color-primary);
  color: var(--color-text-on-primary);
}
```

### ❌ Bad: Tailwind
```tsx
<div className="p-4 bg-gray-100 rounded-md">
  <button className="bg-blue-500 text-white px-4 py-2">Import</button>
</div>
```

---

### ✅ Good: Error Handling (No Unwrap)
```rust
async fn load_projects() -> anyhow::Result<Vec<Project>> {
    let conn = db::connect()
        .context("failed to connect to database")?;

    let projects = conn.query_projects()
        .context("failed to query projects")?;

    Ok(projects)
}
```

### ❌ Bad: Unwrap in Production
```rust
async fn load_projects() -> Vec<Project> {
    let conn = db::connect().unwrap();  // DENIED by Clippy
    conn.query_projects().unwrap()      // DENIED by Clippy
}
```

---

### ✅ Good: Tauri Invoke Pattern
```tsx
async function handleImport() {
  try {
    setLoading(true)
    const result = await invoke<ImportResult>('import_files', {
      source: '/Volumes/SD_CARD',
      destination: '/Users/me/Photos/Project1'
    })
    showNotification(`Imported ${result.fileCount} files`, 'success')
  } catch (err) {
    console.error('Import failed:', err)
    showNotification('Import failed', 'error')
  } finally {
    setLoading(false)
  }
}
```

### ❌ Bad: No Error Handling
```tsx
async function handleImport() {
  const result = await invoke<ImportResult>('import_files', {
    source: '/Volumes/SD_CARD',
    destination: '/Users/me/Photos/Project1'
  })
  // No try/catch - errors crash app
}
```

---

## Maintainer Priorities

**What matters most:**
1. **Styling compliance:** Vanilla CSS only (NO Tailwind, NO CSS-in-JS, NO inline styles)
2. **Data integrity:** File imports with checksum verification, no data loss
3. **User experience:** Clear error messages, progress feedback, responsive UI
4. **Type safety:** Full TypeScript coverage, no `any`

**Trade-offs we accept:**
- More CSS code for maintainability (over Tailwind)
- Explicit error handling for reliability (over terse code)
- Conservative file operations for safety (over speed)

---

## Confidence Threshold

Only flag issues you're **80% or more confident** about.

If uncertain:
- Phrase as question: "Should this use vanilla CSS instead?"
- Suggest investigation: "Consider testing with large files"
- Don't block PR on speculation

---

## Review Tone

- **Constructive:** Explain WHY, not just WHAT
- **Specific:** Point to exact file:line
- **Actionable:** Suggest fix or alternative
- **Educational:** Explain design decisions

**Example:**
❌ "Use vanilla CSS"
✅ "In ImportCard.tsx:42, using Tailwind classes `flex items-center`. This project uses vanilla CSS only (see CLAUDE.md styling section). Create `import-card.css` with:\n```css\n.import-card-header {\n  display: flex;\n  align-items: center;\n}\n```\nThen use: `<div className=\"import-card-header\">`"

---

## Out of Scope

Do NOT review:
- [ ] Formatting (Prettier/rustfmt handles)
- [ ] ESLint/Clippy warnings (CI handles)
- [ ] Import order
- [ ] Personal style preferences

---

## Special Cases

**When PR is:**
- **Styling changes:** Strict vanilla CSS enforcement
- **File operations:** Require checksums, error handling, tests
- **Tauri commands:** Verify Result<T, String>, error context
- **Context addition:** Verify truly cross-cutting (not prop drilling avoidance)

---

## Checklist Summary

Before approving PR, verify:
- [ ] NO Tailwind, CSS-in-JS, or inline styles
- [ ] Vanilla CSS uses design tokens (variables.css)
- [ ] No .unwrap() or .expect() in Rust production code
- [ ] Tauri commands return Result<T, String>
- [ ] Frontend errors shown to user (notifications)
- [ ] File operations have error handling + checksums
- [ ] Tests exist for new code
- [ ] TypeScript: no `any`, proper interfaces
- [ ] React hooks rules followed

---

## Additional Context

**See also:**
- [README.md](../README.md) - Setup, development, CI checks
- [PLAN.md](../PLAN.md) - Architecture, features, requirements
- [.claude/CLAUDE.md](../.claude/CLAUDE.md) - Detailed patterns, styling rules
- [Cargo.toml](../src-tauri/Cargo.toml) - Clippy config
- [eslint.config.js](../eslint.config.js) - ESLint rules

**For questions:** Open issue for architecture/styling questions before implementing

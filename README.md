# CreatorOps

Photography workflow management app built with Tauri 2, React, and TypeScript.

## Features

- Import photos from SD cards with progress tracking
- Automatic metadata extraction (date, camera model, location)
- Project organization with color-coded priorities
- Theme system (light/dark mode with system preference sync)
- Duplicate detection via SHA-256 hashing
- Retry logic for failed imports

## Stack

- **Frontend:** React 19, TypeScript, Vite, Vanilla CSS
- **Backend:** Tauri 2, Rust
- **Tools:** mise for dependency management

## Development

```bash
# Install dependencies
mise install
npm install

# Run dev mode
npm run dev

# Lint & format
mise run fmt
mise run lint

# Test
mise run test

# Build
npm run build
cd src-tauri && cargo build --release
```

## CI Checks

**Frontend:**

```bash
npm run format:check
npm run lint
npm run test -- --run
npm run build
```

**Rust:**

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
mise run lint:rust
mise run test:rust
cd src-tauri && cargo build --release
```

## Architecture

Clean component structure following design guidelines in [.claude/CLAUDE.md](.claude/CLAUDE.md).

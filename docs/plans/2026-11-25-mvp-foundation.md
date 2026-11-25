# CreatorRelay MVP Foundation Implementation Plan

> **For Codex:** Execute the plan task-by-task in the current workspace.

**Goal:** Replace the Tauri starter screen with a polished, functional local-first writing workspace backed by Markdown files and SQLite.

**Architecture:** React presents a three-column desktop writing workspace using reusable Tailwind primitives. Tauri commands own workspace selection, SQLite metadata/indexing, safe Markdown file operations and revision snapshots; the filesystem remains authoritative for document bodies.

**Tech Stack:** Tauri 2, React 19, TypeScript, Tailwind CSS 4, Rust, rusqlite, Tauri dialog plugin, lucide-react, react-markdown, remark-gfm.

---

### Task 1: Frontend tooling and reusable UI foundation

- Add Tailwind CSS 4, lucide-react and Markdown preview dependencies.
- Configure Tailwind's Vite plugin and shared theme tokens.
- Remove Vite starter styles and metadata.
- Build reusable button, icon-button, input, badge and panel styles.

### Task 2: SQLite-backed native workspace

- Add bundled SQLite and directory chooser support to Tauri.
- Persist the selected workspace path in app-local SQLite.
- Add migrations for document index and revision snapshots.
- Implement commands for workspace selection, document list/read/create/save/search and revision history/restore.
- Keep Markdown files authoritative, write atomically, constrain paths to workspace and preserve recoverable revisions.

### Task 3: MVP writing workspace UI

- Implement a calm editorial application shell with navigation, document list and editor/preview.
- Add create/search/select flows and debounced autosave status.
- Add Markdown preview, word/character count and revision history affordance.
- Add workspace onboarding state and error/loading states.

### Task 4: Verification

- Run TypeScript/Vite production build.
- Run Rust formatting and tests/checks for Tauri native commands.
- Review diff and ensure no emoji icons or starter screen remains.

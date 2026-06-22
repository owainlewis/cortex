# Cortex Roadmap

This roadmap turns `docs/prd.md` into implementation tasks.
Each task should be small enough for one focused branch, one review, and one rollback.

## Shared Decisions

- Cortex is macOS only for v1.
- v0.1 proves the minimal single-file editing loop.
- v0.1 uses `ropey::Rope` from the start.
- v0.1 uses simple full-screen redraw behind a renderer abstraction.
- v0.1 implements a tiny Emacs-shaped keyset and a `C-x` prefix state machine.
- v0.1 keeps point and scroll in editor/window view state, not only inside the buffer.
- v0.1 does not include splits, tabs, minibuffer, search, kill ring, tree-sitter, config, AI, or an embedded terminal pane.
- v0.2 keeps one active buffer, but adds directory startup, a small slash command line, visual polish, and first-pass tree-sitter highlighting.

## Milestone: v0.1 Minimal Editor

Project board: [Cortex v0.1](https://github.com/users/owainlewis/projects/13)

GitHub issue tracker:

- [#1 Scaffold the Rust project](https://github.com/owainlewis/cortex/issues/1)
- [#2 Add terminal lifecycle and app shell](https://github.com/owainlewis/cortex/issues/2)
- [#3 Implement rope-backed buffer and file behavior](https://github.com/owainlewis/cortex/issues/3)
- [#4 Implement cursor and viewport movement](https://github.com/owainlewis/cortex/issues/4)
- [#5 Render the single-file editor](https://github.com/owainlewis/cortex/issues/5)
- [#6 Add input, keymap prefix handling, and command dispatch](https://github.com/owainlewis/cortex/issues/6)
- [#7 Add save, status messages, and dirty quit prompt](https://github.com/owainlewis/cortex/issues/7)
- [#8 Manual smoke test and README notes](https://github.com/owainlewis/cortex/issues/8)

### 1. Scaffold the Rust project

**Goal:** Create the initial Rust binary crate and basic project hygiene.

**Context:** The repository currently starts from docs only.
The first implementation task should create the executable shape without building editor behavior yet.

**Relevant files or references:** `docs/prd.md`, `Cargo.toml`, `src/main.rs`, `.gitignore`.

**Proposed approach:** Initialize a Rust binary crate in the repository.
Add dependencies needed for v0.1: `crossterm`, `ropey`, and a small error-handling crate if useful.
Keep the entrypoint simple and return clear CLI errors.

**Acceptance criteria:**
- `cargo run -- --help` or an equivalent usage path returns a clear message.
- Running without a file path exits non-zero with a clear error.
- Running with more than one file path exits non-zero with a clear error.
- `cargo test` runs successfully.
- `.gitignore` covers normal Rust build output.

**Verify:** Run `cargo test` and `cargo run --`.

**Out of scope:** Editor UI, file editing, rendering, and terminal raw mode.

### 2. Add terminal lifecycle and app shell

**Goal:** Enter and leave a macOS terminal editor session cleanly.

**Context:** The editor must use raw mode and the alternate screen, but it must return the terminal to a usable shell after normal exits and common errors.

**Relevant files or references:** `src/main.rs`, `src/terminal.rs`, `docs/prd.md`.

**Proposed approach:** Wrap `crossterm` raw mode and alternate screen setup in a terminal guard.
Create an app loop that can receive events and exit on a temporary key before real command dispatch exists.

**Acceptance criteria:**
- The editor enters the alternate screen.
- The editor exits raw mode and leaves the alternate screen on quit.
- The shell is usable after exit.
- Common setup errors are reported clearly.

**Verify:** Run `cargo test` and manually run `cargo run -- /tmp/cortex-lifecycle.txt`.

**Out of scope:** Editing, save behavior, command registry, and dirty prompts.

### 3. Implement rope-backed buffer and file behavior

**Goal:** Load an existing file or represent a missing file as a clean empty buffer, then save it back to disk.

**Context:** v0.1 uses `ropey::Rope` immediately.
The buffer owns text, dirty state, file path, and save/load behavior.
Cursor and scroll state remain outside the buffer.

**Relevant files or references:** `src/buffer.rs`, `docs/prd.md`.

**Proposed approach:** Add a buffer type with load, insert, delete, dirty tracking, and save operations.
Treat missing files as empty buffers.
Do not create missing parent directories on save.

**Acceptance criteria:**
- Existing files load into the buffer.
- Missing files create an empty clean buffer with the requested path.
- Save writes the buffer contents to disk.
- Save creates the target file when the parent directory exists.
- Save fails clearly when the parent directory does not exist.
- Dirty state changes after edits and clears after save.

**Verify:** Run `cargo test`.

**Out of scope:** Rendering, raw terminal UI, and external file reload.

### 4. Implement cursor and viewport movement

**Goal:** Move point through a rope-backed buffer with predictable Emacs-style line behavior.

**Context:** Movement should work before editing is wired to terminal input.
The view state owns point and scroll so future split windows can view the same buffer independently.

**Relevant files or references:** `src/view.rs`, `src/buffer.rs`, `docs/prd.md`.

**Proposed approach:** Represent point using a rope-friendly cursor position.
Implement forward char, backward char, next line, previous line, start of line, and end of line.
Track a preferred column for vertical movement where practical.

**Acceptance criteria:**
- `C-f` behavior moves forward one character and clamps at EOF.
- `C-b` behavior moves backward one character and clamps at BOF.
- `C-n` and `C-p` move across lines and clamp at file edges.
- `C-a` moves to line start.
- `C-e` moves to line end.
- Movement handles empty files, empty lines, short lines, and final lines without trailing newline.

**Verify:** Run `cargo test`.

**Out of scope:** Terminal key handling and rendering.

### 5. Render the single-file editor

**Goal:** Draw the current file, cursor, and basic status/modeline using a simple full-screen redraw.

**Context:** v0.1 deliberately skips diffed rendering, but the renderer should be isolated so it can be replaced later.

**Relevant files or references:** `src/renderer.rs`, `src/view.rs`, `src/buffer.rs`, `docs/prd.md`.

**Proposed approach:** Add a renderer that takes immutable editor state and paints the visible viewport.
Use a plain modeline or status row showing file name, dirty state, cursor position, and transient messages.

**Acceptance criteria:**
- File contents are visible after opening a file.
- The cursor appears at the editor point.
- Scrolling keeps the point visible.
- The modeline/status row shows file name, dirty state, and cursor position.
- Long lines and small terminal sizes do not panic.

**Verify:** Run `cargo test` and manually run `cargo run -- /tmp/cortex-render.txt`.

**Out of scope:** Syntax highlighting, themes, diffed paint, splits, and tabs.

### 6. Add input, keymap prefix handling, and command dispatch

**Goal:** Route keyboard input through a tiny command system with `C-x` prefix support.

**Context:** The PRD calls for command dispatch and an introspectable command registry later.
v0.1 only needs enough structure to avoid hard-coding all behavior directly in the event loop.

**Relevant files or references:** `src/input.rs`, `src/keymap.rs`, `src/commands.rs`, `docs/prd.md`.

**Proposed approach:** Implement a small key representation, a prefix state machine, and command handlers for the v0.1 keyset.
Printable characters and editing keys can dispatch directly to editing commands.

**Acceptance criteria:**
- Printable characters insert into the buffer.
- Enter inserts a newline.
- Backspace deletes backward.
- Delete deletes forward.
- Arrow keys move point.
- `C-f`, `C-b`, `C-n`, `C-p`, `C-a`, and `C-e` move point.
- `C-x C-s` dispatches save.
- `C-x C-c` dispatches quit or dirty confirmation.
- Prefix state resets after a complete command or invalid key.

**Verify:** Run `cargo test` and manually exercise the keybindings in the editor.

**Out of scope:** Minibuffer, `M-x`, configurable keybindings, search, and kill ring.

### 7. Add save, status messages, and dirty quit prompt

**Goal:** Make file lifecycle safe enough for real use.

**Context:** The minimal editor must not silently lose unsaved edits.
Errors should be visible inside the editor and should not wreck the terminal session.

**Relevant files or references:** `src/app.rs`, `src/commands.rs`, `src/buffer.rs`, `src/renderer.rs`, `docs/prd.md`.

**Proposed approach:** Add save status messages and a dirty quit confirmation state.
Support `y` to quit without saving and `n` or Escape to cancel.

**Acceptance criteria:**
- `C-x C-s` saves the buffer and clears dirty state.
- Save failures show a clear status message and keep the editor open.
- `C-x C-c` exits immediately when the buffer is clean.
- `C-x C-c` prompts when the buffer is dirty.
- In the dirty prompt, `y` exits without saving.
- In the dirty prompt, `n` and Escape cancel the quit.
- Other keys do not accidentally confirm data loss.

**Verify:** Run `cargo test` and manually test clean quit, dirty cancel, dirty confirm, and save failure.

**Out of scope:** Autosave, backup files, external reload, and recursive directory creation.

### 8. Manual v0.1 smoke test and README notes

**Goal:** Confirm the editor works end to end and document the current state.

**Context:** v0.1 is useful only if the basic loop works in a real terminal, not just in unit tests.

**Relevant files or references:** `README.md`, `docs/roadmap.md`, `docs/prd.md`.

**Proposed approach:** Add a short README with install/run notes, current scope, and keybindings.
Run a manual smoke test against a temporary file.

**Acceptance criteria:**
- README explains how to run the editor.
- README lists the v0.1 keybindings.
- README clearly says Cortex is currently macOS-only.
- Manual smoke test confirms open, insert, delete, move, save, dirty quit cancel, dirty quit confirm, and terminal cleanup.
- Any known limitations are recorded briefly.

**Verify:** Run `cargo test` and manually run `cargo run -- /tmp/cortex-smoke.txt`.

**Out of scope:** Marketing copy, screenshots, packaging, and release automation.

## Milestone: v0.2 Current Editor Surface

Project board: [Cortex v0.1](https://github.com/users/owainlewis/projects/13)

Status: complete once issue #21 merges.

GitHub issue tracker:

- [#17 Add directory file picker](https://github.com/owainlewis/cortex/issues/17)
- [#18 Add slash command line](https://github.com/owainlewis/cortex/issues/18)
- [#19 Add visual theme and modeline polish](https://github.com/owainlewis/cortex/issues/19)
- [#20 Add tree-sitter syntax highlighting](https://github.com/owainlewis/cortex/issues/20)
- [#21 Update docs for v0.2 behavior](https://github.com/owainlewis/cortex/issues/21)

Completed behavior:

- Opening a file path starts the editor on that file.
- Opening a directory path starts a picker over non-hidden entries in that directory.
- The picker supports Up, Down, `C-p`, `C-n`, Enter, Escape, and `C-x C-c`.
- The editor has a slash command line with `/open <path>`, `/save`, `/quit`, `/quit!`, and `/help`.
- Unknown slash commands report an error and keep the editor open.
- The editor has the current visual theme, modeline polish, and status styles.
- Tree-sitter highlighting is enabled for Rust, Markdown, JSON, and TOML.
- README and roadmap describe the current v0.2 behavior and limitations.

Still intentionally out of scope for v0.2:

- Multiple buffers, splits, and tabs.
- Minibuffer, search, kill ring, undo, config, plugins, LSP, AI integration, and the embedded terminal pane.
- JavaScript and TypeScript highlighting.
- Directory navigation inside the picker and directory support for `/open`.

## Later Milestones

### v0.3 Editing Feel

- Kill ring with `C-k`, `C-y`, and `M-y`.
- Incremental search with `C-s` and `C-r`.
- Undo and redo.
- Minibuffer foundation.

### v0.4 Windows

- Split layout tree.
- Tabs.
- Per-window modelines and view state where needed.

### v0.5 Agent Workflow

- Manual `revert-buffer`.
- Dirty reload guard.
- Embedded terminal pane using `portable-pty` and `alacritty_terminal`.
- `send-region-to-terminal` context helper.

### Post-v1 Polish

- Diffed rendering.
- Disk-changed marker via `notify`.
- Diff-highlighted reloads.
- Deeper AI integration only if the terminal-pane approach proves insufficient.

# Issue 30 Plan

## Goal

Add multiple in-memory file buffers with Emacs-style find-file and switch-buffer commands.

`C-x C-f` must accept an editable path.
An existing path opens its file.
A missing path opens an empty buffer and saving creates the file when its parent directory exists.

## Implementation

1. Add an editor-level buffer list that owns each buffer and its view state.
2. Keep one active buffer and switch to an already-open path instead of duplicating it.
3. Generalize the existing bottom command input enough to support find-file and switch-buffer prompts.
4. Bind `C-x C-f` to find-file and `C-x b` to switch-buffer.
5. Keep dirty state per buffer, save only the active buffer, and protect dirty inactive buffers on quit.
6. Update the README for the new keys, file creation behavior, and remaining limitations.

## Acceptance Checks

- Opening another file retains the first buffer and its unsaved edits.
- Switching buffers restores the selected buffer and its view state.
- Saving affects only the active buffer.
- Quitting warns when any buffer is dirty.
- `C-x C-f` opens existing files and missing paths.
- Saving a missing path creates the file when its parent exists.
- Saving fails clearly when the parent directory is missing.
- Canceling a prompt leaves the active buffer unchanged.
- Directory startup and picker browsing still work.

## Verification

- Run focused buffer-list, app, keymap, and renderer tests.
- Run `cargo fmt --check`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo test`.
- Manually smoke test find-file, buffer switching, save creation, dirty quit, and terminal cleanup.

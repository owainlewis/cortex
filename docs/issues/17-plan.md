# Issue 17 Plan: Directory File Picker

## Goal

Allow Cortex to accept a directory path and choose one regular file to edit from that directory.

## Current Repo Context

The CLI already accepts exactly one path argument.
`app::run` opens that path as a `Buffer` before entering raw terminal mode.
`TerminalSession` owns raw mode, alternate screen, cursor hiding, and cleanup through `Drop`.
The editor keymap and command dispatch are focused on the single-file editing view.
The renderer already owns full-screen terminal drawing for the editor view.

## Proposed Implementation

Keep normal file opening on the existing `Buffer::open` and editor path.
Detect directories in `app::run` before opening a buffer.
For a directory, read its immediate entries before entering raw mode so unreadable directory errors print normally.
Filter out hidden entries and sort the remaining entries in a stable order.
Show a minimal terminal picker with clear file and directory labels.
Handle Down and `C-n` as next selection.
Handle Up and `C-p` as previous selection.
Handle Enter by opening the selected regular file in the existing editor view.
Handle Escape and `C-x C-c` by quitting without opening a file.
Keep picker state separate from the editor keymap so editing commands do not affect directory selection.
Add focused unit tests for entry filtering and ordering, selection movement, and picker key handling.

## Acceptance Criteria

- `cargo run -- .` opens a directory picker instead of treating the directory as a file.
- The picker lists files and directories clearly.
- Selection can move up and down with arrow keys and `C-n` and `C-p`.
- Enter opens a regular file in the existing editor view.
- Quitting from the picker restores the terminal cleanly.
- Opening a normal file path still works as before.
- Errors for unreadable directories are clear and do not leave the terminal broken.

## Verification Steps

- Run `cargo test`.
- Run `cargo fmt -- --check`.
- Manually run `cargo run -- .` and open a file.
- Manually run `cargo run -- .`, quit from the picker, and confirm the shell is usable.
- Manually confirm opening a normal file path still works.

## Out Of Scope

- Recursive browsing.
- File previews.
- Fuzzy search.
- Creating files from the picker.
- Project tree behavior.
- Multi-file editing.
- Splits.
- Tabs.
- Project indexing.

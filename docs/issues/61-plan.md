# Issue 61 plan

## Task

Remove whole-buffer work from ordinary edit and change-gutter render paths while preserving dirty-state and line-marker behavior.

## Acceptance criteria

- Insert, delete, undo, and redo update dirty state without comparing the complete buffer with its saved contents.
- Undo and redo report clean exactly when they return to the most recently saved edit state.
- Saved line lookup uses the rope index and does not scan saved text from line zero.
- Change markers remain correct for text edits, line insertions and deletions, final-newline changes, save, and reload.
- Large-buffer regression tests exercise editing near EOF and change-marker rendering deep in the file.
- Save, reload, search, and terminal behavior remain unchanged.

## Implementation

1. Track current, clean, and next edit-state identifiers in the buffer.
2. Record the state before and after each edit so undo and redo restore dirty state with constant work.
3. Store the clean baseline as a rope and compare current and saved lines through indexed rope slices.
4. Keep save-race content verification against the same clean rope baseline.
5. Add focused correctness tests and large-buffer guardrails for the two ticketed paths.

## Verification

- Run the focused buffer and renderer tests.
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `cargo test`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo build --all-targets`.
- Manually edit near the end of a large file, save, exit, and confirm terminal restoration.

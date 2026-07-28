# Issue 114: Bound dirty gutter markers

## Task

Replace render-time current-versus-saved line comparisons with deterministic per-buffer change metadata whose lookup cost does not depend on line length.

## Behavior

- The gutter marks lines affected by edits on the current history path from the clean checkpoint.
- Ordinary insertions and deletions update only the affected line metadata.
- Newline insertion and deletion keep markers aligned with the current buffer line indices.
- Undo and redo restore the marker state associated with the restored edit state.
- Returning to the clean history state clears all markers, and moving away from it restores the relevant markers.
- Save establishes the current text as the new clean checkpoint without discarding undo history.
- Reload establishes the reloaded text as the clean checkpoint and clears edit history.
- Each buffer owns independent marker state.
- Renderer lookup uses bounded metadata and never compares complete line contents.
- Recreating saved bytes through a separate new edit remains marked until save or traversal back to the clean history state, matching history-based dirty tracking.

Probabilistic hashes, gutter appearance changes, and broad edit-history redesign are out of scope.

## Implementation

1. Add deterministic line-change metadata beside the existing history-state tracking.
2. Record enough marker state with each edit for forward and inverse application to restore the correct gutter after undo and redo.
3. Reset or rebase the marker checkpoint on save and reload while preserving existing undo-after-save behavior.
4. Replace `Buffer::line_changed` Rope comparisons with bounded metadata lookup.
5. Keep line metadata aligned across edits that add or remove line terminators.
6. Add test-only work accounting for deterministic long-line and deep-viewport guardrails.
7. Track issue #114 as closed in the Editor Quality roadmap section.

## Acceptance criteria

- Rendering a one-million-character visible line performs no work proportional to the hidden line length when deciding its gutter marker.
- Each rebuilt retained frame performs only one bounded metadata lookup per visible text row and never compares line contents.
- Deep viewports do not inspect preceding lines to decide visible markers.
- Insert, delete, newline insertion, newline deletion, and offscreen edits produce correct visible markers.
- Undo to the saved state clears markers, redo restores them, and undo after save marks the restored older state.
- Save and reload reset the marker baseline correctly.
- Multiple buffers keep independent marker state.
- Final-newline-only changes remain marked.
- Existing rendering, highlighting, horizontal clipping, performance fixtures, and terminal cleanup remain unchanged.

## Checks

- Add focused buffer and renderer tests for marker transitions and structural edits.
- Add deterministic million-character and deep-viewport work guardrails.
- Run `cargo fmt -- --check`.
- Run `git diff --check`.
- Run `cargo test`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo build --all-targets`.
- Run `cargo test performance:: -- --ignored --nocapture --test-threads=1`.
- Run a synchronized PTY smoke covering editing, save, undo, resize, exit cleanup, exact terminal-mode restoration, and a usable shell.
- Obtain a fresh independent review and address every important finding.

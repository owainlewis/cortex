# Issue 129: Use Ropey line semantics for gutter ranges

## Goal

Keep changed-line gutter ranges aligned when edits insert or delete any line break that Ropey treats structurally.

## Acceptance criteria

- Changed-line range accounting recognizes LF, CR, CRLF, VT, FF, NEL, line separator, and paragraph separator with Ropey's configured semantics.
- CRLF counts as one structural line break.
- Edits that form or split CRLF at a range boundary use the resulting Ropey line structure.
- Insert, delete, undo, and redo keep marker ranges aligned after non-LF structural edits.
- Saving rebases non-LF structural markers without discarding undo and redo behavior.
- Later marker ranges do not remain at stale indices after non-LF structural edits.
- Existing LF behavior, final-newline markers, and bounded marker lookup behavior remain unchanged.
- File contents remain byte-for-byte as edited and are not normalized.

## Implementation

1. Add focused regressions that expose the existing non-LF structural range drift.
2. Derive edit line-break counts through `RopeSlice` and adjust CRLF boundary contributions from the neighboring rope characters.
3. Keep the existing changed-range transition and final-newline rules unchanged.
4. Add renderer coverage that proves the gutter displays the corrected non-LF range.
5. Apply a focused improve pass to the changed code and tests without widening scope.
6. Review the final diff independently and address every valid finding.

## Checks

- Run focused buffer tests for Ropey line-break accounting and marker transitions.
- Run focused renderer gutter tests.
- Run `cargo fmt --check`.
- Run `cargo test --all-targets`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo build --all-targets --all-features`.

## Out of scope

- Normalizing line endings or file contents.
- Redesigning edit history or changed-line marker storage.
- Changing gutter appearance or renderer architecture.

# Issue 125: Bound retained-frame allocation

## Task

Reject unsupported terminal dimensions at the renderer boundary before Cortex builds editor or picker lines, performs highlighting work, or allocates the retained cell grid.

## Behavior

- Editor and picker rendering accept terminal sizes whose dense retained frame contains at most the documented cell limit.
- Editor and picker rendering return a clear `InvalidInput` error when terminal dimensions exceed that limit.
- Maximum `u16` terminal dimensions are rejected before proportional work or allocation.
- Retained-frame cell-count arithmetic is checked.
- Retained-frame allocation uses a fallible reservation path so allocation failure returns an error and unwinds through terminal cleanup.
- Zero, tiny, ordinary, Unicode, resize, and retained-frame diff behavior remain unchanged for supported dimensions.

Clamping, sparse retained frames, renderer redesign, and platform compatibility work are out of scope.

## Implementation

1. Define and document one maximum retained-frame cell count in the renderer.
2. Validate terminal dimensions at the public editor and picker render boundary before other size-proportional work.
3. Use checked cell-count arithmetic and fallible retained-grid reservation.
4. Keep invalid-size errors descriptive and recoverable through the existing `io::Result` path.
5. Add pure editor and picker regressions for maximum `u16` dimensions.
6. Add boundary tests at and immediately above the retained-cell limit.
7. Add resize coverage across a rejected size without corrupting retained valid-frame state.
8. Add synchronized PTY coverage proving editor and picker rejection restores raw mode, the alternate screen, and the cursor.

## Acceptance criteria

- Editor and picker return a clear recoverable error for maximum `u16` dimensions without attempting the giant allocation.
- The retained-frame cell count has a documented deterministic upper bound.
- Capacity arithmetic and retained-grid allocation cannot panic, overflow, or abort for any terminal dimensions accepted by the renderer.
- The exact retained-cell limit is accepted and the next cell above the limit is rejected.
- Invalid dimensions are rejected before editor highlighting or editor and picker line construction.
- Zero, tiny, ordinary, resize, wide-grapheme, and retained-cell behavior remain unchanged.
- Terminal cleanup runs after an oversized editor or picker render error.

## Checks

- Run focused renderer tests for editor and picker maximum dimensions, limit boundaries, resize recovery, Unicode, and retained-cell behavior.
- Run synchronized PTY oversized-size cleanup tests for editor and picker.
- Run the existing synchronized PTY signal cleanup tests.
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `cargo test --all-targets`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo build --all-targets`.
- Apply the `improve` skill to the changed code without changing intended behavior.
- Obtain a fresh independent subagent review and address every important finding.

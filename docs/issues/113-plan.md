# Issue 113: Keep point visible on long lines

## Task

Add one per-view horizontal terminal-cell offset that keeps point visible while long lines remain clipped.

## Behavior

- Point stays inside the text viewport after horizontal or vertical movement, editing, search, undo, reload, buffer switching, and terminal resize.
- Moving right advances the horizontal offset only when point reaches the viewport edge.
- Moving left reveals earlier text and returns the offset to zero when point returns to the line start.
- The offset is stored in `View`, so each buffer keeps its own horizontal position.
- The offset and cursor position use terminal cell columns.
- Horizontal clipping starts and ends only at complete extended grapheme cluster boundaries.
- Tabs retain four-column tab stops, wide graphemes retain two cells, and standalone zero-width clusters retain one visible editor cell.
- Modeline line and column values continue to describe the buffer position rather than the screen position.

Soft wrapping, configurable scroll margins, and horizontal scroll settings are out of scope.

## Implementation

1. Add a horizontal cell offset to `View` and extend point-visibility maintenance to accept the text viewport width.
2. Normalize the offset to a grapheme boundary when the required left edge falls inside a tab, wide grapheme, or other multi-cell cluster.
3. Add a bounded, revision-aware line-column checkpoint cache so repeated frames and nearby movement do not re-segment hidden prefixes.
4. Add Rope helpers that return a visible line window without cloning the hidden prefix or suffix.
5. Render and highlight the visible window with its original buffer character, byte, and terminal-column origins.
6. Position the terminal cursor relative to the horizontal offset while leaving modeline columns unchanged.
7. Preserve the offset in each existing per-buffer `View` and recompute visibility on resize.
8. Track #113 in the Editor Quality roadmap section.

## Acceptance criteria

- Moving or editing beyond the right edge scrolls horizontally and keeps point visible.
- Moving back left reveals earlier content and eventually restores column zero.
- Line start, line end, vertical movement, search, undo, reload, resize, and buffer switching keep a coherent horizontal offset.
- ASCII, tabs, combining marks, ZWJ emoji, flags, skin tones, variation selectors, wide CJK, and standalone zero-width clusters are never split by clipping or cursor placement.
- Long lines remain clipped and are not soft-wrapped.
- Existing vertical scrolling, syntax highlighting, retained rendering, modeline behavior, and terminal cleanup remain unchanged.

## Checks

- Add focused pure tests for right and left scrolling, editing, line start and end, vertical movement, resize, buffer switching, tabs, wide graphemes, and zero-width graphemes.
- Add a structural guardrail proving repeated deep horizontal frames do not re-segment hidden prefixes.
- Reproduce the current pinned-cursor bug with the focused tests and a synchronized narrow-terminal PTY smoke before the fix.
- Run `cargo fmt -- --check`.
- Run `git diff --check`.
- Run `cargo test`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo build --all-targets`.
- Repeat the synchronized PTY smoke after the fix and verify editing, resize, terminal restoration, and shell usability.
- Obtain a fresh independent review and address every important finding.

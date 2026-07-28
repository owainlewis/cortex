# Issue 50 Plan

## Task

Replace ordinary full-screen redraws with retained terminal cell diffing behind the existing renderer boundary.
Preserve current editor behavior and macOS terminal polish.

## Implementation

1. Build styled terminal cell frames for the editor and directory picker.
2. Retain the last successfully flushed frame inside `Renderer`.
3. Emit only changed cell runs during ordinary rendering.
4. Invalidate the retained frame on resize and after returning from the nested directory picker.
5. Use a safe sequential full-redraw path when context-dependent graphemes make cached scalar columns unreliable.
6. Keep cursor visibility, style resets, alternate-screen cleanup, and shell restoration unchanged.

## Acceptance Criteria

- Ordinary cursor movement does not clear the screen or rewrite unchanged buffer text.
- First paint, resize, picker entry, and picker return repaint the complete frame when required.
- Syntax colors, selection, gutter, modeline, command line, picker, and cursor placement remain correct.
- Context-dependent graphemes and regional-indicator flags do not use unsafe changed-cell coordinates.
- The terminal cursor and alternate screen are restored after exit.
- Focused pure-logic tests cover changed-cell skipping, resize invalidation, and safe grapheme fallback behavior.

## Checks

- Run `cargo fmt -- --check`.
- Run `git diff origin/main...HEAD --check`.
- Run `cargo test --no-fail-fast`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo build --all-targets --all-features`.
- Run a synchronized PTY smoke covering ordinary movement, resize, picker handoff, exit cleanup, and a usable shell marker.

## Out of Scope

Complete grapheme-aware cellization remains in issue #63.
Async rendering, alternate backends, soft wrap, new visual features, and multi-pane rendering are out of scope.

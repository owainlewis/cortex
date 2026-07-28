# Issue 63 Plan

## Task

Make extended grapheme clusters the user-visible editing and rendering unit.
Preserve current macOS editor behavior while ensuring Unicode text is never split at a user-perceived character boundary.

## Product Policy

Point remains stored as a Rope character index, but it may rest only at an extended grapheme cluster boundary.
Horizontal movement advances or retreats by one extended grapheme cluster.
Backward and forward deletion remove one complete extended grapheme cluster.
Mark and region endpoints are grapheme boundaries, so selection, cut, yank, undo, and redo never split a cluster.
Vertical movement preserves the preferred terminal cell column and chooses the last grapheme boundary that does not pass that column.
Horizontal clipping includes a complete grapheme cluster or excludes it.
Combining marks, variation selectors, emoji modifiers, regional-indicator flags, and ZWJ sequences remain attached to their cluster.
Terminal width comes from the maintained `unicode-width` implementation and is applied to the complete cluster.
Normal text clusters occupy the reported one or two terminal cells.
A standalone cluster reported as zero cells occupies one editor cell so it remains visible and cursor-addressable.
Tabs keep the existing four-column tab-stop behavior.
Control clusters keep the existing visible-space behavior.

## Implementation

1. Add maintained Unicode grapheme segmentation and terminal-width dependencies.
2. Add small shared helpers for grapheme boundaries, display width, clipping, and terminal-column lookup.
3. Make `View` movement and point clamping grapheme-safe.
4. Make backward and forward deletion operate on complete grapheme ranges.
5. Keep insertion, region cutting, yanking, undo, redo, search, and reload points on grapheme boundaries.
6. Build renderer segments and retained cells from complete grapheme clusters.
7. Remove the temporary context-dependent full-redraw fallback once grapheme cell coordinates are reliable.
8. Document the policy in the PRD and mark issue #63 complete in the roadmap after implementation is verified.

## Acceptance Criteria

- Decomposed accents render with their combining marks and move or delete as one unit.
- ZWJ emoji, regional-indicator flags, skin-tone sequences, and variation-selector sequences move, select, delete, undo, and render as complete units.
- Wide CJK graphemes occupy two cells and are never clipped halfway.
- Cursor placement uses complete-cluster widths after movement, editing, selection, and renderer transitions.
- Vertical movement never places point inside a grapheme cluster.
- Diff rendering skips unchanged grapheme cells and safely transitions between Unicode frames.
- Existing ASCII editing, tabs, syntax styles, picker behavior, resize handling, and terminal cleanup remain unchanged.

## Checks

- Run focused pure tests for combining marks, ZWJ emoji, flags, skin tones, variation selectors, wide CJK, selection, delete, undo, cursor placement, clipping, and renderer transitions.
- Run `cargo fmt -- --check`.
- Run `git diff origin/main...HEAD --check`.
- Run `cargo test --no-fail-fast`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo build --all-targets --all-features`.
- Run a synchronized PTY smoke that edits and navigates representative Unicode text, resizes, exits, and confirms shell restoration.

## Out of Scope

Unicode normalization is out of scope.
Word and sentence movement are out of scope.
Bidirectional text layout is out of scope.
Soft wrapping and horizontal scrolling are out of scope.
Configurable Unicode width policies are out of scope.

# Issue 20 Plan

## Goal

Add tree-sitter syntax highlighting for common source files while preserving Cortex's custom renderer and plain rope-backed buffer.

## Current repo context

The buffer stores plain text in `ropey::Rope` and owns file loading, dirty state, and saving.

The view owns point and scroll state.

The renderer builds visible screen lines from immutable buffer and view state, then paints terminal cells with the current built-in theme.

Issue #19 added the current restrained truecolor palette in `src/renderer.rs`.

The app redraws synchronously on input and resize.

## Proposed implementation

Add a focused `src/highlighter.rs` module around `tree-sitter-highlight`.

Support Rust, Markdown, JSON, and TOML first.

Detect languages from file extension.

Highlight only the visible lines used by the renderer.

Return plain text spans when a language is unknown, parsing fails, or a query cannot be used.

Keep highlight spans out of `Buffer`, `View`, editing commands, saving, and file behavior.

Map highlight categories to the renderer theme so styling does not affect text layout or cursor placement.

## Acceptance criteria

Known file types render with syntax highlighting.

Unknown file types render as plain text.

Highlighting does not change buffer contents or cursor movement behavior.

Large or invalid files do not panic.

Highlighting is isolated behind renderer and highlighter boundaries.

Existing editing, save, dirty quit, and terminal cleanup behavior still works.

## Verification steps

Run `cargo fmt -- --check`.

Run `cargo test`.

Manually open highlighted Rust, Markdown, JSON, and TOML files.

Manually edit and save a highlighted file.

Manually confirm the shell is usable after quitting.

## Out of scope

LSP.

Semantic highlighting.

User themes.

Config files.

Tree-sitter editing commands.

Folding.

Search.

Multi-buffer highlight caches.

Background parsing.

Splits.

Tabs.

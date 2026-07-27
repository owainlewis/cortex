# Issue 62 Plan

## Goal

Keep syntax colors correct when the viewport starts inside a multiline construct whose opener is above the visible rows.

## Current repo context

The buffer owns rope-backed text and each editor buffer has its own view state.

The renderer owns one syntax highlighter and currently sends only width-clipped visible lines to it.

The highlighter joins those lines into a standalone document, so Tree-sitter cannot see Rust block comment or multiline string openers and Markdown fence openers above the viewport.

The editor already supports multiple buffers, and future windows may render the same buffer through more than one view.

## Proposed implementation

1. Add a monotonic text revision to `Buffer` that changes after insert, delete, undo, and redo operations that modify text.
2. Keep document highlight caches in `SyntaxHighlighter`, keyed by stable buffer identity and revision.
3. Parse and highlight the source prefix needed for the viewport plus bounded line read-ahead and per-line character limits when its cache is absent, stale, or too short.
4. Return only the document-relative line spans requested by the renderer.
5. Keep buffer contents, view state, editing behavior, and terminal styling unchanged.
6. Add focused tests for Rust block comments, Rust multiline strings, and Markdown fenced Rust code when their openers are above the requested viewport.
7. Add cache tests showing that scrolling reuses a parsed document and editing invalidates only the edited buffer's cached highlights.

## Acceptance criteria

Rust block comments and multiline strings retain their syntax colors after their opening delimiter scrolls above the viewport.

Markdown fenced blocks retain block styling and injected Rust highlighting after the fence opener scrolls above the viewport.

Scrolling and repeated rendering do not reparse an unchanged buffer.

Editing invalidates the changed buffer's cached document without invalidating other buffers.

Highlighting a large buffer near its start does not parse or retain the unrelated tail.

Highlighting a minified or otherwise very long physical line does not parse its unrelated tail beyond Cortex's visible editing surface.

Highlight state remains behind the renderer and highlighter boundary and is reusable by multiple views of the same buffer.

Unknown file types remain plain text, and invalid source remains safe.

## Verification

Run focused highlighter and renderer tests.

Run `cargo fmt --check`.

Run `cargo clippy -- -D warnings`.

Run `cargo test`.

Run `cargo build --release`.

Manually open and scroll through large Rust and Markdown multiline constructs, edit them, quit Cortex, and confirm the shell remains usable.

Review the final diff with a fresh subagent and address every important finding before publishing.

## Out of scope

Background parsing.

Changes to syntax queries or theme colors.

Moving syntax state into `Buffer`.

Window or split implementation.

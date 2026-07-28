# Issue 112 plan

## Task

Bound repeated syntax-highlighting work at deep Rust and Markdown viewports without losing multiline or injected-language context.

## Current behavior

The highlighter caches one highlighted prefix per buffer revision.

A deep first request builds and parses that prefix once.

Every edit invalidates the complete prefix, and scrolling beyond the cached end parses the unchanged prefix again.

The merged #51 guardrail reproduces this with a near-end Rust and Markdown viewport.

## Implementation

1. Record the start line of the latest buffer change with its before and after revisions.
2. Keep compact lexical context checkpoints for Rust and Markdown at fixed line intervals.
3. Parse a bounded line window around the requested viewport, seeded from the nearest checkpoint so block comments, multiline strings, Markdown fences, and fenced Rust keep their context.
4. Invalidate checkpoints and the cached window from the affected checkpoint forward while retaining unrelated buffer caches.
5. Retain only one bounded highlight window and a fixed number of recent checkpoints per buffer.
6. Preserve the existing prefix cache for other supported languages.
7. Extend the #51 deep-viewport guardrail to include a near-viewport edit and refresh.

## Acceptance criteria

- A deep Rust or Markdown edit does not rebuild or parse the prefix from line zero.
- Parser input after a warmed deep edit is bounded by the viewport window and per-line character limit.
- Context scanning after a warmed deep edit is bounded by checkpoint and window sizes.
- Scrolling within a cached window does not reparse, and scrolling beyond it extends from a recent checkpoint.
- Rust block comments, ordinary and raw multiline strings, Markdown fences, and fenced Rust highlighting remain correct.
- One buffer edit does not invalidate another buffer's cache.
- Retained highlight lines and context checkpoints have deterministic caps independent of total file length.
- Unknown file types and other supported languages preserve their current behavior.

## Checks

- Run focused highlighter tests, including deep Rust and Markdown edits.
- Run `cargo test performance:: -- --ignored --nocapture --test-threads=1`.
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `cargo test`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo build --all-targets`.
- Run a large-file PTY save and exit smoke and confirm exact terminal restoration.
- Obtain fresh independent review and address every important finding.

## Out of scope

- Background parsing.
- LSP or semantic highlighting.
- Replacing Tree-sitter or its highlight queries.
- A general incremental parser framework.
- Changes to rendering, themes, or editor commands.

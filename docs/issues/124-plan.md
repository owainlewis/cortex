# Issue 124 plan

## Task

Bound synthetic Rust block-comment parser context independently of file depth and nested opener count without changing ordinary Rust or Markdown highlighting behavior.

## Current behavior

Rust context checkpoints retain block-comment nesting as a fixed-size `usize`.

Rebuilding a deep highlight window repeats one synthetic opener and closer for every retained nesting level.

The resulting Tree-sitter request can therefore allocate and parse bytes in proportion to all nested openers before the viewport.

Unchecked nesting increments can also overflow in debug builds at the integer limit.

## Implementation

1. Add a documented fixed cap for exact block-comment nesting and its synthetic parser wrapper.
2. Represent deeper nesting with a fixed-size overflow state that conservatively classifies Rust as commented until an edit allows context to be rebuilt below the cap.
3. Stop the Markdown overflow fallback at the surrounding fence boundary so normal Markdown highlighting resumes after the fence.
4. Add a failing regression that reproduces prefix-proportional request growth with many nested openers and a deep viewport.
5. Cover exact-bound closure behavior, excessive-nesting fallback, and edits that add, remove, or close nesting on both sides of the cap.
6. Exercise the same bounded fallback through Markdown fenced Rust.
7. Preserve the existing checkpoint and highlight-window invalidation design.

## Acceptance criteria

- Rust parser input added by block-comment context is capped by a documented constant independent of file depth and nested opener count.
- Retained Rust syntax context remains fixed-size.
- Nested block comments at or below the cap retain exact context and can close inside a rebuilt viewport.
- Excessive nesting uses a deterministic conservative fallback without panicking, overflowing, or allocating in proportion to the file prefix.
- Edits before and inside a deep viewport invalidate affected checkpoints and rebuild within the same bound.
- Markdown fenced Rust, prior viewport context behavior, and the #62 and #112 bounds remain unchanged.

## Checks

- Run focused highlighter tests for request bounds, closure correctness, Markdown fenced Rust, and edits around the cap.
- Run `cargo test performance:: -- --ignored --nocapture --test-threads=1`.
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `cargo test --all-targets`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo build --all-targets`.
- Inspect the final diff for scope and behavior preservation.
- Obtain fresh independent subagent review and address every important finding.

## Out of scope

- A parser framework or background highlighting.
- LSP or semantic highlighting.
- Replacing Tree-sitter or changing highlight queries.
- Broad syntax-context or checkpoint architecture changes.

# Issue 146 plan

## Task

Preserve injected Rust highlighting when an oversized line splits a normal Markdown fence into bounded parser segments.

## Current behavior

Windowed Markdown highlighting retains fence, quote-depth, and injected-language context across truncated lines.

The parser still treats every segment after a truncated-line barrier as a standalone Markdown document.

Without a synthetic fence opener, Rust after the barrier is highlighted as ordinary Markdown until a later window rebuild starts beyond the barrier.

## Implementation

1. Keep the existing truncated-line barriers and context checkpoints.
2. Parse each bounded window segment with the syntax context active at that segment's first line.
3. Advance the retained context across each segment, treating its final oversized line as a barrier before reseeding the next segment.
4. Preserve real Markdown fence closers, quote-depth rules, and ordinary Markdown after a closer.
5. Leave the #124 overflow fallback and #142 barrier policy unchanged.

## Acceptance criteria

- Rust after an oversized line inside an unquoted Rust fence keeps injected-Rust highlighting in the same rebuilt window.
- The equivalent block-quoted Rust fence keeps its quote-depth and injected-Rust context.
- A real closer after the barrier ends injected highlighting on the correct line.
- Markdown after the closer remains Markdown.
- Parser input, scanned work, retained checkpoints, and cached window lines stay within the existing bounds.
- Existing truncated-line barriers, #124 nested-comment overflow behavior, and #142 overflow recovery remain unchanged.

## Checks

- Add focused unquoted and block-quoted fenced-Rust regressions with an oversized middle line.
- Cover a real closer and a following Markdown heading in both forms.
- Assert request, scan, retained-window, and checkpoint bounds.
- Run `cargo test highlighter::tests:: -- --nocapture`.
- Run `cargo test performance:: -- --ignored --nocapture --test-threads=1`.
- Run `cargo fmt --check`.
- Run `git diff --check origin/main...HEAD`.
- Run `cargo test --all-targets`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo build --all-targets`.
- Apply the `improve` workflow without changing behavior.
- Obtain fresh independent subagent review and address every important finding.
- Push a ready pull request that closes #146 and wait for protected checks and current review feedback.
- Reply on issue #146 and the original PR #144 thread with commit, pull-request, and verification evidence.
- Resolve the PR #144 thread only when the published fix fully addresses it.

## Out of scope

- A new parser framework or complete CommonMark parser.
- Changes to the renderer, themes, editor commands, or language queries.
- Changes to checkpoint retention, request limits, or overflow policy.

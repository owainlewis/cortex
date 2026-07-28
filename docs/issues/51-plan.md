# Issue 51 plan

## Task

Add opt-in local performance checks for representative large-buffer editor work without introducing flaky CI timing gates.

## Acceptance criteria

- One documented command runs all local performance checks.
- Generated fixtures cover repeated rope insert and delete work, large-file viewport rendering, visible Rust and Markdown highlighting, and simple large-buffer search.
- Pass and fail decisions use deterministic correctness or structural output bounds.
- Elapsed times are diagnostic only and are never universal thresholds.
- Normal `cargo test` does not run the slower checks.
- The checks require no network, committed large fixture, service, or new dependency.
- The deterministic hot-path tests merged for #61 remain unchanged.

## Implementation

1. Add a test-only performance module with ignored checks for the four required workloads.
2. Generate all large inputs in temporary files and remove them after each check.
3. Print elapsed time for local before-and-after comparison while asserting only stable outcomes.
4. Document the command, workload sizes, interpretation, and non-goals.
5. Link the local checks from contributor guidance and update roadmap status.

## Verification

- Run `cargo test performance:: -- --ignored --nocapture --test-threads=1`.
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `cargo test`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo build --all-targets`.
- Run a PTY save and exit smoke and confirm the shell is restored.

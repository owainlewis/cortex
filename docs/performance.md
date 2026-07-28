# Local performance checks

Cortex keeps local performance checks small, repeatable, and separate from normal CI.

Run all checks serially with:

```sh
cargo test performance:: -- --ignored --nocapture --test-threads=1
```

The checks generate their own temporary large files and cover:

- repeated insertion and deletion near the end of a large rope-backed buffer
- repeated viewport rendering deep inside a large file
- visible-line syntax highlighting near the end of large Rust and Markdown files
- repeated search for a match near the end of a large buffer

Each check asserts deterministic correctness or a structural output bound.
It also prints elapsed time so changes can be compared on the same Mac under similar conditions.
Elapsed times are diagnostic and are not universal pass or fail thresholds.
Use repeated runs and investigate order-of-magnitude changes rather than small timing differences.

The checks require no network access, external service, or committed large fixture.
Normal `cargo test` compiles but does not run them.

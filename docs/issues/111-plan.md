# Issue 111 plan

## Task

Restore terminal state when Cortex receives SIGTERM or SIGHUP without performing terminal I/O inside a signal handler.

## Current context

`TerminalSession` already restores the cursor, alternate screen, and raw mode through `Drop`.
The editor and directory picker block in `crossterm::event::read`, while default signal termination bypasses Rust destructors.
A synchronized PTY reproduction confirms that SIGTERM and SIGHUP leave raw terminal flags active and emit neither the alternate-screen leave nor cursor-show sequence.

## Implementation

1. Register SIGTERM and SIGHUP with `signal-hook` actions that only store the signal number atomically.
2. Poll terminal events with a short timeout so editor and picker loops can observe the signal without unsafe handler I/O.
3. Let the normal stack unwind through `TerminalSession::drop`.
4. Remove the temporary handlers and return a non-zero interrupted result after cleanup.
5. Add synchronized PTY coverage for editor and picker shutdown paths.

## Acceptance criteria

- SIGTERM and SIGHUP restore canonical input, echo, the alternate screen, and cursor visibility.
- Signal handling performs no terminal I/O or allocation inside the handler.
- The editor and picker both leave through the existing RAII cleanup path.
- Dirty in-memory edits are not written to disk during signal shutdown.
- Repeated termination signals do not panic or deadlock.
- Normal quit, dirty-quit confirmation, and setup-error cleanup remain unchanged.

## Verification

- Run the synchronized pre-fix and post-fix PTY signal checks.
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `cargo test`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo build --all-targets`.
- Manually compare exact PTY terminal flags before and after SIGTERM and SIGHUP.

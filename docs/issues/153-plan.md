# Issue 153: Exit after controlling PTY disconnect

## Task

Make Cortex exit promptly when its controlling PTY disappears instead of remaining alive in a high-CPU terminal input loop.

## Current context

Cortex calls `crossterm::event::poll` on the main thread so it can check SIGTERM and SIGHUP between terminal events.
Crossterm's Unix reader can loop forever after PTY hangup because it retries EOF and unhandled terminal read errors without reaching its timeout.
That traps Cortex inside the dependency, prevents its signal checks from running, and leaves one orphaned process consuming one CPU core.
The bug reproduces with both Crossterm 0.27 and 0.29, so reverting the dependency upgrade is not a fix.

## Implementation

1. Arm one terminal disconnect guard at the start of `TerminalSession::enter`, before raw mode and alternate-screen setup.
2. Monitor terminal stdin for `POLLHUP`, `POLLERR`, and `POLLNVAL` without requesting or consuming input events.
3. Leave the guard inactive when stdin is redirected or explicitly opened from `/dev/tty` because macOS `poll` rejects that descriptor and no other standard descriptor is guaranteed to refer to the same terminal.
4. Never use redirected stdout for the hard-exit path.
5. Exit immediately when the PTY controller is gone because Crossterm may have trapped the main thread and the destroyed PTY can no longer receive cleanup output.
6. Keep the guard owned by `TerminalSession` so every terminal path is covered and connected cleanup remains paired with the monitor.
7. Add synchronized PTY regressions for startup disconnect, active-session disconnect, and closed redirected stdout.
8. Keep the existing signal, resize, input, dirty-buffer, renderer-error, and connected terminal cleanup behavior unchanged.
9. Serialize the test-only `openpty` to child-spawn window so parallel tests cannot inherit a controller before `FD_CLOEXEC` is set.

## Acceptance criteria

- Closing the PTY master causes Cortex to exit within one second without requiring SIGTERM.
- A disconnected Cortex process does not remain alive in a sustained high-CPU loop.
- SIGTERM and SIGHUP still unwind through terminal cleanup.
- Editor and picker key input and resize events still use the existing main-thread Crossterm loop.
- Redirected stdin does not make the disconnect guard bypass connected-terminal cleanup.
- Explicit `/dev/tty` stdin does not make the disconnect guard bypass connected-terminal cleanup.
- Closing redirected stdout does not make the disconnect guard bypass cleanup for terminal stdin.
- Normal quit restores the terminal and leaves the shell usable.
- Renderer failures still restore connected terminal state.
- No dependency fork or vendored dependency is required.

## Checks

- Run the new PTY disconnect regression before and after the fix.
- Run the existing synchronized PTY signal and cleanup tests.
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `cargo test --all-targets`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo build --all-targets`.
- Run a manual PTY smoke that edits, exits normally, verifies exact terminal restoration, and reuses the shell.
- Sample the post-disconnect process state to confirm Cortex exits instead of spinning.
- Obtain a fresh independent review and address every important finding.

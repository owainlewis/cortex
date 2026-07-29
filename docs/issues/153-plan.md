# Issue 153: Exit after controlling PTY disconnect

## Task

Make Cortex exit promptly when its controlling PTY disappears instead of remaining alive in a high-CPU terminal input loop.

## Current context

Cortex calls `crossterm::event::poll` on the main thread so it can check SIGTERM and SIGHUP between terminal events.
Crossterm's Unix reader can loop forever after PTY hangup because it retries EOF and unhandled terminal read errors without reaching its timeout.
That traps Cortex inside the dependency, prevents its signal checks from running, and leaves one orphaned process consuming one CPU core.
The bug reproduces with both Crossterm 0.27 and 0.29, so reverting the dependency upgrade is not a fix.

## Implementation

1. Add one terminal disconnect guard after each successful terminal setup.
2. Monitor the inherited terminal on stdout for `POLLHUP`, `POLLERR`, and `POLLNVAL` without requesting or consuming input events.
3. Keep redirected stdin out of the guard because Crossterm may open `/dev/tty` for input and macOS `poll` rejects that opened descriptor.
4. Exit immediately when the PTY controller is gone because Crossterm may have trapped the main thread and the destroyed PTY can no longer receive cleanup output.
5. Stop and join the monitor before ordinary `TerminalSession` cleanup on every connected exit path.
6. Add a synchronized PTY regression that closes the controller after the initial frame and requires Cortex to exit within one second.
7. Keep the existing signal, resize, input, dirty-buffer, renderer-error, and connected terminal cleanup behavior unchanged.
8. Serialize the test-only `openpty` to child-spawn window so parallel tests cannot inherit a controller before `FD_CLOEXEC` is set.

## Acceptance criteria

- Closing the PTY master causes Cortex to exit within one second without requiring SIGTERM.
- A disconnected Cortex process does not remain alive in a sustained high-CPU loop.
- SIGTERM and SIGHUP still unwind through terminal cleanup.
- Editor and picker key input and resize events still use the existing main-thread Crossterm loop.
- Redirected stdin does not make the disconnect guard bypass connected-terminal cleanup.
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

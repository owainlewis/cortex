# Issue 175: Reliable macOS PTY disconnect monitoring

## Problem

The disconnect monitor requested no poll events.
A real macOS PTY probe showed that this mask misses controller closure, while `POLLIN` reports `POLLIN | POLLHUP`.
The editor often exited through a later write error, masking the problem until highlighting startup exceeded the existing one-second integration deadline.
A diagnostic run also observed `ttyname_r` returning `ERANGE` for terminal stdin, silently disabling monitoring.

## Change

Request read readiness and retain the explicit hangup/error checks.
Sleep after ordinary readiness without consuming input, so a busy editor cannot make the monitor spin.
Compare stdin's device identity with `/dev/tty` to preserve exclusion of the special macOS device without a device-name lookup.
Propagate unexpected classification errors before terminal setup begins.

## Acceptance and proof

A real PTY regression test must observe controller hangup with the production polling function.
Create test PTY descriptors with atomic close-on-exec flags so concurrent metadata commands cannot inherit them.
The hangup test keeps an unrelated child alive to verify that isolation.
A second test must show that readable input remains intact for the editor and requests throttling.
Keep the integration test's one-second deadline and all redirected-input cleanup coverage.
Run focused terminal tests, the full suite, formatting, Clippy, a release build, and a real shell-restoration check.
Require fresh review and passing GitHub checks before merging.

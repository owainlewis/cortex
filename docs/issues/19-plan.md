# Issue 19 Plan

## Goal

Add a restrained built-in visual theme and polish the modeline/status treatment.
The editor should feel intentional in normal editing, dirty state, save success, save failure, prefix, command-line, picker, and dirty quit states.
The rendering should stay small, hand-rolled, and inside the existing renderer boundary.

## Current repo context

`Renderer` owns full-screen drawing for the editor and directory picker.
The editor viewport and the bottom chrome are currently plain terminal output with reverse-video modelines.
`AppState` owns transient status, dirty quit prompt state, and slash command-line state.
`commands::dispatch` already reports save success and save failure through `CommandOutcome`.
The directory picker has its own status message and prefix state.
The current renderer already has tests for tiny terminal sizes, long lines, cursor placement, command-line cursor placement, and picker selection.

## Proposed implementation

Add a tiny internal theme/chrome layer in `src/renderer.rs`.
Use crossterm truecolor colors directly.
Paint every editor and picker row, including empty rows after end of file, so empty space looks deliberate.
Replace the reverse-video modeline with styled segments for file name, dirty/clean state, cursor position, and status text.
Make save success, save failure, prefix, dirty quit, command-line, picker status, and plain informational messages visually distinct.
Keep cursor placement based on the existing frame cursor calculation.
Keep command-line and picker cursor placement exact.
Pass a small render status kind from `app.rs` instead of teaching the renderer about app internals.
Add or update focused renderer and app tests for the new visual states and empty-space behavior.

## Acceptance criteria

- The modeline has a deliberate visual treatment using terminal colors.
- Dirty, clean, save success, save failure, prefix, and dirty quit states are visually distinguishable.
- Command-line and picker status states are visually distinguishable.
- Empty editor space is painted intentionally after the end of the file and on small terminals.
- Cursor placement remains correct after rendering.
- Small terminal sizes and long lines do not panic.
- The shell is restored cleanly after exit.

## Verification steps

- Run `cargo fmt -- --check`.
- Run `cargo test`.
- Manually inspect clean, dirty, saved, failed-save, prefix, command-line, picker, and dirty-quit states in a PTY.
- Manually check a small terminal size if practical.
- Manually confirm the shell is usable after exiting.

## Out of scope

User themes are out of scope.
Config files are out of scope.
`ratatui` and other UI frameworks are out of scope.
Syntax highlighting is out of scope.
File picker redesign is out of scope.
Splits and tabs are out of scope.
Diffed rendering is out of scope.

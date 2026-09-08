# Issue 165: Literal terminal paste

## Task and acceptance

Enable bracketed paste during terminal setup and disable it before restoring the cursor, alternate screen, and raw mode.
If enabling paste mode writes successfully but flushing fails, still attempt to disable it during cleanup.
Route Crossterm paste events as text data, bypassing individual key handling and automatic indentation.
Insert a nonempty paste at point as one undo step, clear the active mark, and cancel any pending key prefix.
An empty paste leaves text and history unchanged but still cancels a pending prefix.

In editable prompts, convert CRLF, other line breaks, and tabs to spaces, with one space per CRLF pair.
Drop other control characters while preserving ordinary text and spaces.
Pasting never submits a prompt or answers a dirty-quit confirmation.
Ignore pasted text in the directory picker and cancel any pending picker key prefix.
Buffer text retains pasted control characters literally; rendering continues to sanitize them.

## Checks

Test Unicode, tabs, LF/CRLF, control bytes, literal insertion into an indented line, mark clearing, empty paste, and one-step undo/redo.
Test each prompt, dirty confirmation, prefix cancellation, and partial paste-mode setup failure.
Extend terminal checks to verify paste-mode cleanup and a real bracketed-paste/edit/save/undo session.
Run the full suite, formatting, all-target Clippy, release build, independent review, and a real terminal smoke with shell restoration.

# Issue 167: Explicit macOS clipboard commands

## Task and acceptance

Add copy-region, with clipboard-copy as an alias, bound to M-w.
Copy the active region without changing text, mark, history, or the local kill ring.
An empty region produces a clear no-op status and does not invoke a clipboard program.
Add clipboard-paste, bound to C-c C-v, inserting literal text as one undo edit with the same Unicode and mark rules as terminal paste.
The clipboard shortcut also works inside editable prompts, applying the existing single-line conversion without submitting them.
Clipboard commands cannot answer a dirty-quit confirmation.

Use a small adapter with fixed /usr/bin/pbcopy and /usr/bin/pbpaste programs.
Access the clipboard only when an explicit copy or paste command executes.
Bound process I/O and exit waiting by a two-second deadline, avoid blocking pipe reads or writes, and reap failed or timed-out children.
Limit one clipboard transfer to 16 MiB; report oversized data or invalid UTF-8 without editing the buffer.
Do not include clipboard contents in failure messages.

## Checks

Use controlled executable adapters in automated tests, never the real clipboard.
Test copy/no-region behavior, named aliases and keybindings, literal paste and undo, prompt conversion, mark and kill-ring preservation, process failures, invalid UTF-8, oversized data, and timeout cleanup.
Run formatting, strict Clippy, the full suite, release build, independent review, and a terminal copy/paste/edit/undo/save/quit session with shell restoration.
If manual testing changes the system clipboard, save and restore its prior bytes without displaying them.

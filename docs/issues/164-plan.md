# Issue 164: Practical indentation

## Task and acceptance

Tab inserts spaces to the next four-column display stop when no region is active.
Enter copies the current line's leading spaces and tabs, limited to whitespace before point.
It uses the current line's LF or CRLF ending, falling back to the previous line at an unterminated final line, then LF for a new file.
Tab and Shift-Tab change all selected lines as one undo step and preserve the selected range for repeated indentation.
A region ending at the start of a line excludes that line.
Shift-Tab without a region removes up to four columns of leading indentation from the current line.
Do not split graphemes or convert existing tabs while carrying or removing indentation.

## Implementation

Keep line and Rope access in Buffer and point in View.
Add bounded leading-whitespace and line-ending queries to Buffer.
Reuse one contiguous replacement for a selected block so existing history records the entire operation as one edit.
End the replacement at the last changed prefix, so single-line operations retain only indentation bytes in history.
Map point and mark through inserted or removed prefixes; leave an unchanged outdent as a no-op.
Named indent and outdent commands share their keybinding behavior.

## Checks

Cover tab stops after wide characters and tabs, indentation split points, CRLF and mixed line endings, region boundaries and direction, blank lines, mixed whitespace, graphemes, and undo/redo point restoration.
Run focused command/app/input tests, the full suite, formatting, all-target Clippy, and release build.
Use a real PTY to type nested code, indent and outdent a region, undo, save, and verify terminal restoration and a usable shell.

## Verification

The final local suite passed 352 unit tests and eight terminal integration tests.
The two redirected-input tests remain restricted by the local sandbox; require their unchanged CI coverage before merge.
Formatting, all-target Clippy, and the release build passed.
A real release PTY flow typed nested Rust using Tab, Enter, and Shift-Tab, saved it, indented and outdented a region ending at a line boundary, exercised one-step undo/redo across saves, and restored a usable shell.
Independent review approved after replacing whole-line indentation history with prefix-only changes.
A one-million-character regression proves that single-line indent/outdent retains only the four changed bytes in history.

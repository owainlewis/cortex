# Issue 168: Word, page, buffer, and line navigation

## Task and acceptance

Add M-f/M-b word movement and M-d/M-Backspace word kills through the existing kill ring.
Words are Unicode alphanumeric runs plus underscore, with combining marks kept in their containing grapheme.
Forward movement skips separators then reaches the end of a word; backward movement skips separators then reaches its beginning.
Word kills use those same boundaries and one undo edit, clearing the region after a successful cut.

Add M-</M-> for buffer start/end.
Add C-v/PageDown and M-v/PageUp for paging with two lines of overlap and at least one line of movement.
Store the last rendered viewport height in View, preserve the preferred display column, and keep point visible at edges and after resize.
Add goto-line with a positive one-based argument through M-x; clamp beyond EOF and reject zero, negative, overflow, and nonnumeric input without moving point.
All commands appear in the registry and end typing/yank-pop groups as deliberate actions.

## Checks

Test Unicode words, punctuation, underscores, combining marks, emoji separators, empty buffers, EOF, and long lines.
Retain grapheme context in both directions and bound requested context bytes over growing regional-indicator fixtures.
Compare retained traversal against flat Unicode segmentation across rope chunk boundaries.
Test forward/backward kills with yank and undo, buffer endpoints, page overlap and edges, preferred columns, and all input mappings.
Test goto-line validation and named-command/key equivalence where applicable.
Run formatting, strict Clippy, the full suite, release build, independent review, and a real Unicode navigation/kill/yank/page/goto/save/quit session with shell restoration.

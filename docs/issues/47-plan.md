# Issue 47: Kill ring and yank-pop

## Task and acceptance

Replace the single cut slot with a newest-first ring of at most 32 nonempty entries.
Line and region kills push complete text into the ring; do not coalesce separate kills.
C-y inserts the newest entry at point as one edit.
M-y replaces that yank with the next older entry, wrapping through the ring while the yank interaction remains valid.
With one entry, yank-pop leaves history unchanged and reports that there is no older entry.
An empty ring or invalid yank-pop reports a clear status without editing text.

Track the exact inserted character range, buffer identity, revision, and ring index for a yank.
Use the exact range even when inserted combining characters join adjacent graphemes; preserve neighboring text and keep the displayed point valid.
Movement, editing, buffer changes, undo/redo, paste, selection commands, and other completed commands invalidate yank-pop.
Keep command-prompt entry available for the named yank-pop command without moving point or editing buffer text.
Each kill, yank, and yank-pop has its own undo boundary.
Preserve dirty state and saved-baseline behavior, and clear region state after successful edits.
Keep the ring in process memory with no clipboard or persistence work.

## Checks

Test newest-first order, 32-entry eviction, empty cuts, and complete retained text.
Test repeated cycling, exact Unicode replacement at both grapheme seams, one-entry and empty-ring no-ops, mark clearing, and undo/redo across saves.
Test invalidation after movement, edit, buffer switch, paste, and history commands, plus named-command access.
Run formatting, strict Clippy, the full test suite, release build, independent review, and a real kill/yank/yank-pop/undo/redo/save/quit session with shell restoration.

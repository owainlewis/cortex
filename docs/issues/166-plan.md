# Issue 166: Useful undo groups

## Task and acceptance

Group contiguous ordinary typing and same-direction character deletion when consecutive edits are less than 750 ms apart.
Pass explicit timestamps into the pure dispatch path so timing tests do not sleep.
Movement, prefix keys, prompts, buffer switches, saves, history commands, and deliberate edits end the active group.
Generic buffer insertion and replacement remain separate edits, including paste, indentation, kills, and yanks.
Undo applies every edit in the group in reverse order and restores its original point; redo applies the group in order.
Keep each edit's structural line-change metadata and history identity so Unicode and saved baselines remain correct.

Count retained inserted and deleted UTF-8 bytes across undo and redo.
Evict oldest whole groups above 16 MiB, preserving the newest group even if it alone exceeds the budget.
New editing discards only the redo branch; eviction never changes clean-state identity.

The release smoke exposed a pre-existing undo shortcut defect: Crossterm decodes the legacy `0x1f` byte sent for `C-/` and `C-_` as `C-7`.
Accept that decoded alias and prove it through a real PTY regression.

## Checks

Test typing, both deletion directions, direction changes, 749/750 ms boundaries, noncontiguous points, grapheme merging, and restored points.
Test deliberate-command, prompt, movement, save, and buffer-switch boundaries through application dispatch.
Test dirty state and changed-line markers through grouped Unicode edits, save, undo, redo, and eviction.
Use a small injected history budget for deterministic whole-group eviction tests and check the production limit.
Run formatting, strict Clippy, the full suite, release build, a real terminal editing session with shell restoration, and a fresh independent review.

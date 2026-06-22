# Issue 27 Plan

## Goal

Add a small undo and redo system so editing is recoverable without changing the single-buffer architecture.

## Current repo context

The buffer owns rope-backed text, dirty state, file loading, and saving.

The view owns point, scroll, and preferred column.

Editing commands currently mutate the buffer directly and then update point.

The keymap already resolves direct keys and `C-x` prefixed commands.

The slash command line can dispatch named editor actions.

## Proposed implementation

Store undo and redo stacks in `Buffer`.

Record each text insertion or deletion as a small edit containing the changed range, inserted text, deleted text, and before/after point positions.

Clear redo history when a new edit is made after undo.

Keep dirty state content-correct by comparing the current buffer text with the last saved text.

Expose `undo` and `redo` methods on `Buffer` that return the point position the view should move to.

Add `Undo` and `Redo` commands.

Bind undo to `C-/` and `C-_`.

Expose `/undo` and `/redo` slash commands so redo is available without inventing an awkward early keybinding.

## Acceptance criteria

Undo reverses text insertion.

Undo reverses newline insertion.

Undo reverses backward and forward deletes.

Redo reapplies an undone edit.

A new edit after undo clears redo history.

Point moves to a sensible location after undo and redo.

Dirty state remains correct after undo, redo, and save.

Existing editing, save, quit, command line, picker, and highlighting behavior still works.

## Verification steps

Run `cargo test`.

Run `cargo check`.

Manually open a file, type, delete, undo, redo, save, and quit.

Manually confirm the shell is usable after exit.

## Out of scope

Emacs-style undo tree.

Persistent undo history.

Multi-buffer undo coordination.

Visual undo UI.

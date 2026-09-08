# Cortex product specification

Status: target behavior for a macOS-only local terminal editor.
The delivery batch is [#160](https://github.com/owainlewis/cortex/issues/160).
This document describes intent, not proof of shipped behavior.
Use the README for current commands and the roadmap for implementation status.

## 1. Product direction

Cortex is a small, modeless code editor with Emacs-style keys.
Build for the author's own macOS coding workflow first.
The daily loop is opening files, writing code, navigating and changing text, saving, and checking edits made by other tools.
The editor should feel fast, look considered, and remain small enough to understand.

Cortex owns text editing and multiple file buffers with one visible editor view.
A terminal multiplexer such as tmux, or the host terminal's panes, owns side-by-side shells, coding agents, and process sessions.
Internal splits, tabs, and terminal emulation are deferred.
Separate Cortex processes do not share unsaved buffers or undo history.
Revisit internal views only when simultaneous views of the same unsaved text become a demonstrated need.

## 2. Principles and boundaries

Prefer one predictable interaction for each operation.
Use curated defaults before adding settings.
Do not add Windows or Linux support, LSP, plugins, scripting, AI provider APIs, or a broad configuration system in this delivery batch.
The source tree remains one Rust binary crate unless a verified implementation need requires otherwise.

Input latency, Unicode correctness, safe saving, terminal cleanup, and useful error messages are product requirements.
Colour and spacing should direct attention to code, point, selection, and state that needs action.
Avoid permanent panels, decorative badges, patched-font requirements, and repeated branding.

## 3. Core ownership and invariants

A `Buffer` owns Ropey text, file identity and baseline, revision, changed-line markers, and undo/redo history.
A `View` owns point, scroll, and preferred display column.
The editor owns the buffer collection and active view; transient interaction state belongs to the application or a dedicated prompt/search component.
Only the main application loop mutates editor state.
Rendering reads editor state through a renderer boundary and retains the last successfully flushed cell grid.

Point, mark, deletion, movement, selection, and history must respect extended grapheme clusters.
Rope character indices describe editing positions; terminal columns describe display positions.
Existing tabs use four-column stops.
Long lines scroll horizontally without splitting a displayed grapheme.

Saving preserves the existing atomic-save, permissions, metadata, symlink, and disk-baseline safeguards.
A failed save leaves text and dirty state intact.
Reload refuses to discard unsaved edits.
Quitting or closing a dirty buffer requires explicit confirmation.
An inactive dirty buffer receives the same protection as the active buffer.
Missing files can be opened and created by saving when their parent directory exists.

## 4. Typing, paste, and history

Tab inserts spaces to the next four-column stop.
With a region active, Tab indents its lines and Shift-Tab outdents them as one edit.
A selection ending at the start of a line excludes that line.
Enter carries leading whitespace from the current line, limited to whitespace before point when splitting indentation.
Preserve existing tabs while carrying indentation; do not run a language formatter implicitly.

Bracketed terminal paste inserts literal text as one undo unit.
Tabs, newlines, and command-like text in a paste never dispatch keybindings.
Prompt paste edits prompt text without submitting it; each prompt defines its single-line conversion rule.
Enable and disable terminal paste mode as part of the same lifecycle guarantees as raw mode and the alternate screen.

Group contiguous typing and same-direction deletion into useful undo steps.
A pause of at least 750 milliseconds, movement, save, prompt entry, buffer switch, or a deliberate edit command ends a typing group.
Do not merge history across a saved baseline.
Indentation, paste, kills, yanks, and replacement have explicit undo boundaries.
Keep a linear undo/redo history and discard the redo branch after new editing.
Limit retained undo/redo text payload to 16 MiB per buffer by evicting whole oldest edit groups.
Retain the newest group even when that single group exceeds the limit so the last edit remains undoable.

## 5. Commands, prompts, and navigation

Use a small static registry with stable command names and descriptions.
`M-x` accepts registered names and exposes available commands.
Keybindings and named commands execute the same behavior.
Retain slash command aliases for compatibility while supporting ordinary command names without a leading slash.
Invalid commands and cancelled prompts never edit the buffer.

Find-file and switch-buffer show filtered candidates with disambiguating paths.
Find-file searches the nearest enclosing Git project, falling back to the startup directory, and respects ignored files and generated directories.
Do not follow directory symlinks during project traversal.
Literal existing or missing paths remain available for opening and creating files.
File discovery is bounded and cancellable so a large project cannot monopolize input.
Closing a buffer preserves other buffers and asks before discarding dirty text.

Word movement treats Unicode alphanumeric runs and underscore as words.
Page movement uses the visible viewport with a small overlap.
Go-to-line accepts a positive one-based line number.
All movement keeps point visible and on a valid grapheme boundary.

## 6. Search and replacement

`C-s` starts forward incremental search and `C-r` starts reverse search.
Typing a query moves point to a visible current match without editing text.
Repeating the search key advances in its direction.
Enter accepts a match; Escape cancels and restores the original point and view.
Show no-match state without unexpectedly moving point.
Literal search must avoid unnecessary whole-buffer copies on each keystroke.

`query-replace` asks for literal search and replacement text, then walks forward from point without wrapping.
Confirm each match with `y`, skip with `n`, replace remaining matches with `!`, or stop with Escape or `C-g`.
An empty query is rejected, and replacement text containing the query cannot cause an infinite loop.
Each confirmed replacement is undoable; replacing all remaining matches is one undo unit.
Regular expressions and project-wide replacement are outside this batch.

## 7. Kill ring and macOS clipboard

A bounded kill ring retains recent cuts for `C-y` and `M-y`.
Yank-pop replaces the previous yank only while that interaction is still valid.
Intervening edits, movement, and buffer changes must not allow stale ranges to replace unrelated text.
Copy-region retains text without deleting it.

Clipboard commands are explicit local operations using macOS clipboard tools.
`M-w` copies the active region and `C-c C-v` pastes the system clipboard.
Clipboard failures leave buffer text intact and show an actionable status.
Do not inspect, synchronize, persist, or transmit clipboard contents automatically.
Tests use controlled clipboard adapters.

## 8. Syntax highlighting and visual surface

Retain Tree-sitter for Rust, Markdown, JSON, TOML, Python, JavaScript/JSX, TypeScript/TSX, Ruby, and OCaml.
Unknown types remain editable plain text.
TypeScript includes ordinary JavaScript captures, and TSX includes JSX captures.
Markdown formatting must not reinterpret source inside fenced code blocks.
Multiline syntax context remains correct across viewport boundaries.

Cache parsing by buffer identity and revision.
Avoid repeatedly copying and parsing an ever-growing file prefix while typing near the bottom of a supported file.
Bound retained work and reject stale results after editing, reload, or switching buffers.
Verify correctness and work bounds with fixtures; report elapsed timings as diagnostics rather than universal machine thresholds.

Ship one coherent truecolor dark palette.
Use absolute line numbers, a subdued gutter, and distinct current-line, selection, and search treatments.
The compact modeline prioritizes file identity, unsaved or external-change state, point position, and language.
Errors, confirmations, and prompts take priority at narrow terminal widths.
Cursor placement, redraws, and styles must remain correct after resize and horizontal scrolling.

## 9. External tools and terminal lifecycle

Coding agents and shells run in separate terminal or tmux panes and exchange file changes through the filesystem.
The active buffer's disk-change indicator updates while idle within the one-second polling interval plus event-loop tolerance.
Use bounded metadata checks; disk polling must not copy the file contents or reload text automatically.
`C-x C-r` reloads a clean buffer and preserves a useful point position.

Document tmux truecolor support and its default `C-b` prefix conflict with backward-character movement.
Do not modify the user's terminal configuration or run a terminal multiplexer inside Cortex.
Terminal tests use dedicated temporary sessions and sockets.

Normal quit, handled signals, input errors, render errors, and partial startup restore the shell's terminal modes and cursor.
The existing PTY-disconnect behavior remains intentional when no terminal controller remains.

## 10. Target bindings

These are target bindings; consult the README for those currently implemented.

| Key | Command |
| --- | --- |
| `C-x C-s`, `C-x C-c` | Save; quit with dirty guard |
| `C-x C-f`, `C-x b`, `C-x k` | Find/create file; switch buffer; close buffer |
| `C-x C-r` | Reload clean buffer |
| `C-f`, `C-b`, `C-n`, `C-p` | Character and line movement |
| `C-a`, `C-e` | Line start and end |
| `M-f`, `M-b`, `M-<`, `M->` | Word and buffer movement |
| `C-v` / PageDown, `M-v` / PageUp | Page movement |
| `M-d`, `M-Backspace` | Kill forward/backward word |
| Tab, Shift-Tab, Enter | Indent, outdent, indented newline |
| `C-Space`, `C-w`, `C-k` | Mark, kill region, kill line |
| `C-y`, `M-y` | Yank, yank-pop |
| `M-w`, `C-c C-v` | Copy region to clipboard; clipboard paste |
| `C-/`, `C-_`, `C-x u` | Undo |
| `C-s`, `C-r` | Forward/reverse incremental search |
| `M-x` | Named command, including redo, goto-line, and query-replace |

## 11. Delivery and proof

The ordered tickets and their dependencies live in [#160](https://github.com/owainlewis/cortex/issues/160) and [roadmap.md](roadmap.md).
Each behavior change needs focused pure-logic tests and relevant terminal proof.
Run formatting, Clippy, the complete test suite, and a release build before accepting the batch.
Run the documented performance checks and a complete coding session directly and in an isolated tmux session.
Verify shell usability after exit and confirm documentation against merged code.
Publishing a release and replacing the installed binary remain separate release decisions.

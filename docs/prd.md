# Cortex — Design Specification

*An AI-native, Emacs-flavored, terminal-native text editor in Rust.*

**Status:** Ready for v0.1 implementation planning.
**Audience:** the author (solo build), and any future contributor.
**One-line pitch:** A stunning, lightweight, fast terminal editor with Emacs keybindings — where the AI agent is simply a real terminal running in a pane, not a protocol to integrate.

---

## 1. Vision

Cortex is a terminal text editor for someone who wants the *feel* of Emacs — modeless chord-driven editing, a kill ring, a minibuffer, point-and-mark — without the forty years of accumulated weight, and who has decided that an AI model, not a language server, is their source of code intelligence.

It is deliberately small. It is not trying to out-feature VS Code or out-ecosystem Emacs. It is trying to be the editor *one specific person* daily-drives because it is fast, looks good, behaves like Emacs where it matters, and treats a coding agent (e.g. Claude Code) as a first-class collaborator rather than a chat panel bolted onto the side.

The taste benchmark is Ghostty: fast, elegant, native-feeling, minimal-but-complete. Cortex should feel like it belongs in that lineage.

---

## 1a. Design Principles

These are the rules that settle arguments. When a feature or a decision is in doubt, it loses to these.

**Subtract first.** Every feature is guilty until proven essential. The non-goals list (§2) is not a disclaimer — it is the design. A smaller editor is faster, more legible, and more beautiful by default. The terminal-pane approach to AI (§8) was chosen over a protocol precisely because it *removes* a subsystem instead of adding one.

**Latency is the headline feature.** Keypress-to-photon is the number that matters. Diffed paint (touch only changed cells), no speculative work on the input path, render on demand. If a feature threatens input latency, it is redesigned or dropped. This is non-negotiable because it's the first thing the body notices and the hardest to retrofit.

**Beauty is cheap in a terminal — spend on the right things.** "Stunning" here does not mean more UI; it means *restraint executed precisely*. The levers are nearly free computationally: generous padding and breathing room, a restrained truecolor palette, tree-sitter highlighting, a beautiful modeline, a crisp cursor, and zero flicker. Lightweight and gorgeous are not in tension — in a terminal they're the same discipline. Chrome, panels, and badges are the enemy of both.

**One way to do things.** One interaction model (Emacs chords), one config file, one mental model per concept. Coherence is a feature; optionality is a tax. No modes, no second input paradigm, no plugin layer to reason about.

**Reuse the primitive you already have.** The cell grid renders buffers, splits, the modeline, *and* the embedded terminal. The agent integration is "another pane," not new infrastructure. Prefer designs where the new capability falls out of an existing mechanism.

**Native muscle memory, no surprises.** Emacs keybindings behave exactly as a muscle-memoried Emacs user expects, or they aren't claimed. Half-right bindings are worse than absent ones.

---

## 2. Goals and Non-Goals

### Goals
- **Terminal-native.** Runs in any modern terminal. No GUI, no Electron, no web view.
- **Fast.** Sub-frame input latency. Flicker-free, diffed rendering. Big files stay snappy.
- **Emacs feel.** Chord sequences, kill ring, minibuffer, point/mark, incremental search.
- **Aesthetic.** Truecolor themes, tree-sitter highlighting, a real modeline, considered spacing and cursor.
- **AI by composition, not integration.** The agent runs in an ordinary terminal pane and edits files on disk; file panes reflect the changes. No protocol, no server — the simplest thing that works, and it inherits everything the agent can already do.
- **Windowed.** In-editor splits and tabs, so source and the agent's pane sit side by side without leaving Cortex.
- **Coherent core.** A clean command-dispatch core with an introspectable command registry (powers `M-x` and key remapping) — but deliberately *not* a programmable scripting runtime.

### Non-Goals (v1)
- **No LSP.** Intelligence comes from AI. (Tree-sitter stays — it is independent of LSP and drives highlighting.)
- **No scripting runtime.** No embedded Lua/Scheme, no config-as-program. Behavior is configured declaratively, not reprogrammed live. Cortex keeps Emacs *ergonomics*, not Emacs *programmability*.
- **No plugin ecosystem.** No package manager, no community registry. Features ship in-tree.
- **Not a built-in multiplexer.** Cortex has in-editor splits and tabs (see §6.9), but it is not trying to replace tmux for session/process management — just window layout within one editor.
- **Not a drop-in Emacs.** Cortex borrows Emacs's *ergonomics*, not its internals or its package universe.
- **macOS only.** Cortex is designed for the author's macOS terminal workflow. Cross-platform support is not a v1 goal.

---

## 2a. v0.1 Build Contract

v0.1 is intentionally tiny.
It should prove the core editing loop before adding windows, search, tree-sitter, config, or the embedded terminal pane.

**Goal:** `cargo run -- path/to/file` opens a macOS terminal editor that can view, edit, save, and quit one file.

**Included in v0.1:**
- A normal Rust binary crate in this repository.
- Raw terminal mode and alternate screen handling via `crossterm`.
- A single file buffer backed by `ropey::Rope`.
- Simple full-screen redraw through a small `Renderer` abstraction.
- Printable text insertion, newline insertion, Backspace, and Delete.
- Cursor movement with `C-f`, `C-b`, `C-n`, `C-p`, `C-a`, `C-e`, and arrow-key aliases.
- `C-x C-s` to save.
- `C-x C-c` to quit, with a dirty-buffer confirmation prompt.
- Basic status/modeline text showing file name, dirty state, cursor position, and transient errors.
- Focused unit tests for buffer editing, cursor movement, dirty tracking, save behavior, and prefix-key resolution.

**Explicitly excluded from v0.1:**
- Splits and tabs.
- Minibuffer.
- Incremental search.
- Kill ring.
- Tree-sitter highlighting.
- Config file.
- Embedded terminal pane.
- AI integration.
- Diffed rendering.
- Cross-platform terminal behavior.

**File behavior:**
- The CLI requires exactly one file path.
- Existing files open into a rope buffer.
- Missing files open as empty clean buffers associated with that path.
- Saving may create the target file.
- Saving must not create missing parent directories.
- Save errors are shown in the editor and do not exit the process.

---

## 3. Target User and Use Case

A developer who:
- currently uses Ghostty + (VS Code and/or Emacs/Neovim),
- relies on an AI coding agent for navigation/completion/refactoring,
- values speed and aesthetics and has the taste to notice when they're missing,
- wants to *watch and guide* an agent editing code, live, in the same place they edit.

Primary loop: open files, edit with Emacs muscle memory, let Claude Code make changes on disk, see those changes reflected in Cortex, and steer via shared context.

---

## 4. Architecture Overview

Cortex is structured as a small core with clearly separated subsystems, communicating through a central **command dispatch** seam. Everything — a keypress, an `M-x` invocation, a window split, a file reload — resolves to a *command* executed against editor state.

The AI integration is not a subsystem at all. It is **a real terminal running in one of the window panes**, with a coding agent (e.g. Claude Code) inside it. The agent edits files on disk; the file panes reload to show the changes. The "integration" is the window system plus the on-disk filesystem — nothing more.

```
            ┌─────────────────────────────────────────────┐
            │                  Terminal                    │
            │            (raw mode via crossterm)          │
            └───────────────┬───────────────┬─────────────┘
                            │ key events    │ draw
                            ▼               ▲
        ┌───────────────────────────────────────────────────┐
        │                  Input / Keymap                    │
        │     prefix-key trie → resolves to a Command        │
        └───────────────┬───────────────────────────────────┘
                        ▼
        ┌───────────────────────────────────────────────────┐
        │              Command Dispatch (core seam)          │
        │     keypress | M-x | window op | reload            │
        └───┬───────────┬───────────┬───────────────────────┘
            ▼           ▼           ▼
        ┌───────┐  ┌─────────┐  ┌──────────────────────────┐
        │Buffer │  │Minibuf  │  │   Window Layout (tree)    │
        │(ropey)│  │/search/ │  │  ┌─────────┐ ┌─────────┐  │
        │KillRng│  │ nucleo  │  │  │ file    │ │ terminal│  │
        └───┬───┘  └─────────┘  │  │ window  │ │ pane    │  │
            │                   │  │(buffer) │ │ (PTY +  │  │
            ▼                   │  │         │ │  agent) │  │
        ┌───────────────┐       │  └─────────┘ └────┬────┘  │
        │  Renderer      │      └───────────────────┼───────┘
        │ (diffed paint, │◄─ tree-sitter            │ writes files
        │  cell grid)    │   highlight              ▼  on disk
        └───────┬────────┘                  ┌──────────────┐
                │ file panes reload ◄────────│  filesystem  │
                └───────────────────────────└──────────────┘
```

Both a *file window* and a *terminal pane* are leaves in the same window-layout tree, drawn by the same cell-grid renderer. That symmetry is the whole trick.

---

## 5. Technology Stack

| Concern | Choice | Why |
|---|---|---|
| Terminal I/O | `crossterm` | Raw mode, key events, 24-bit truecolor, cursor control, cross-platform. |
| Rendering | hand-rolled on crossterm | Editors need fine cursor control + minimal diffed repaints. (`ratatui` optional for chrome only — modeline/minibuffer.) |
| Text buffer | `ropey` | O(log n) edits and slices on million-line files; cheap to view-slice for rendering. |
| Syntax highlighting | `tree-sitter` + `tree-sitter-highlight` | Incremental, fast, LSP-independent; the core aesthetic win. |
| Fuzzy finding | `nucleo` | Helix's matcher; powers find-file and buffer switching. |
| File watching | `notify` | inotify/FSEvents/etc. for the (optional, later) auto-reload path. |
| Diffing | `similar` | Map cursor/point through external edits; render agent changes as diffs. |
| Embedded terminal | `portable-pty` + `alacritty_terminal` | Run a shell/agent in a pane. `portable-pty` spawns the PTY; `alacritty_terminal` is a reusable terminal core (parser + grid) so you don't hand-write the `vte` callback layer. |
| Window layout | no new dep (own a layout tree on `crossterm`) | Splits/tabs are a layout tree + viewport partitioning; the renderer already targets cell regions. |
| Config | `toml` (+ `serde`) | Static, declarative config: keybindings, themes, settings. No scripting runtime. |

**Rendering note:** Render directly with crossterm. `ratatui` is built for widget dashboards doing full-screen redraws; the editing surface wants byte-precise cursor placement and a diffed paint that only touches changed cells. Use `ratatui` (if at all) for non-editing chrome.

---

## 6. Core Subsystems

### 6.1 Buffer
- Backed by a `ropey::Rope`. (A `Vec<String>` is acceptable for the very first prototype, but `ropey` is the intended default and the migration is cheap.)
- Tracks: the rope, a dirty flag, the file path, and file metadata needed for save/reload behavior.
- Point, mark, and scroll offset belong to the window/editor view state, not solely to the buffer.
- Multiple buffers held in a buffer list; one is active.
- Undo/redo: an edit-history stack (group keystrokes into sensible undo units; Emacs-style undo can come later).

### 6.2 Renderer
- **Diffed paint.** Maintain a shadow of the last drawn screen; only emit escape codes for cells that changed. This is the difference between flicker and butter, and it directly serves the latency goal.
- Viewport: scroll offset + visible range derived from point.
- Line wrapping handled explicitly (soft-wrap toggle is a later nicety).
- Generous padding; block/bar cursor styles; truecolor throughout.

### 6.3 Input / Keymap — the prefix-key trie
Emacs bindings are chord *sequences* (`C-x C-s`, `C-x C-c`, `C-x C-f`), not a flat map. Model the keymap as a **trie / little state machine**:
- A leaf resolves to a Command.
- An internal node (e.g. after `C-x`) puts the editor in a *pending-prefix* state; the next key resolves against the sub-map.
- The minibuffer shows the pending prefix (`C-x-`) so the user isn't flying blind.
- Modeless by default (no Vim-style modes). This is a defining Cortex choice.

### 6.4 Command Dispatch
- Every action is a named Command (`save-buffer`, `kill-line`, `isearch-forward`, `revert-buffer`, `ai-edit-region`, …).
- Keybindings, `M-x`, hooks, and AI actions all funnel through the same dispatch.
- Commands are introspectable (this is what makes `M-x` and declarative key remapping work).

### 6.5 Kill Ring
- Not a clipboard — a **ring**. A `VecDeque` with a cursor.
- `C-k` kills onto the ring, `C-y` yanks the top, `M-y` cycles through history (only valid immediately after a yank).
- Optional bridge to the system clipboard as a separate, explicit affordance.

### 6.6 Minibuffer
- The prompt line at the bottom that drives `M-x`, find-file, and search.
- Hosts incremental completion (via `nucleo`) for commands, files, and buffers.
- Shows transient messages and the pending key-prefix.

### 6.7 Point and Mark
- `C-Space` sets the mark; the **region** is everything between point and mark.
- Region drives kill, copy, and AI-on-region operations.
- Visible region highlight.

### 6.8 Incremental Search (isearch)
- `C-s` searches *as you type*, jumping live to matches; `C-s` again advances.
- The single most Emacs-feeling interaction in the editor — prioritize getting it smooth.
- (`C-r` is reverse isearch, which is why the reload command does **not** claim `C-r` — see §8.3.)

### 6.9 Window Management — splits and tabs
Cortex shows multiple buffers at once via in-editor splits, and groups layouts via tabs. This is a real subsystem, kept distinct from the buffer list (Emacs's lesson: *windows* show *buffers*, and the two are decoupled — many windows can show one buffer, and a buffer can be shown in zero windows).

**Model**
- A **window** is a viewport onto a buffer, with its own point and scroll offset. (Note: per-window point means cursor/scroll live on the window, not solely the buffer — adjust §6.1 so a buffer can be viewed by several windows independently.)
- A **layout** is a binary tree of splits: each internal node is a horizontal or vertical split with a ratio; each leaf is a window. One leaf is **focused**.
- A **tab** is a saved layout (a whole window tree). A tab bar lists them; switching tabs swaps the active layout. One frame, many tabs.

**Window content is polymorphic.** A leaf window renders one of a small set of content types — primarily a *file window* (a viewport onto a buffer) or a *terminal pane* (a PTY + terminal grid, §8). Both are just cell grids the renderer paints into a rectangle. Keeping the leaf generic over content type is what lets the AI agent live in the layout for free (§8).

**Rendering**
- Partition the screen rectangle by walking the layout tree; draw each window into its sub-rectangle with a 1-cell divider between siblings.
- The diffed paint already works per-cell, so multi-window is "render N viewports into N rects" — no new rendering model, just rectangle bookkeeping. A terminal pane is the same: its grid is just another source of cells.
- Optional thin tab bar at the very top; modeline renders per-window (or one global modeline for the focused window — decide during build).

**Focus and commands**
- Input routes to the focused window. For a file window it edits the buffer; for a terminal pane it writes to the PTY. Window-navigation commands move focus or restructure the tree.
- Resizing: grow/shrink the focused split; equalize. (A resize also sends `SIGWINCH`/new size to any terminal pane.)
- Classic Emacs window commands (`C-x 2/3/0/1/o`) and tab commands (`C-x t …`) — see §11.

**Why it matters for the AI workflow:** splits let you keep source in one pane and the agent running in a terminal pane beside it — watching Claude Code edit a file while you sit in the file's view, all without tmux. The window system *is* the AI integration surface.

---

## 7. Syntax Highlighting

- `tree-sitter` per-language grammars + `tree-sitter-highlight`.
- Kept despite dropping LSP: highlighting is independent of LSP, it's incremental (so it's fast), and it is *the* thing that makes a terminal editor look modern.
- Themes map highlight capture names → truecolor.
- Incremental re-parse on edit so large files don't restall.

---

## 8. AI Integration — a terminal in a pane

The core thesis, restated for v0.3: **Cortex does not integrate with an AI agent. It runs one in a pane.** A coding agent like Claude Code is already a terminal program that reads and edits your codebase. So the entire "integration" is: open a terminal pane (§6.9), run the agent in it, and let your file windows reflect the edits it makes on disk. No protocol, no server, no exposed editor internals.

This is the minimalist move and it is *better*, not just smaller. It reuses the window system and the filesystem you already have, and it inherits every capability the agent already has — codebase search, multi-file edits, tool use — for free. The one thing it gives up (the editor telling the agent your live cursor/selection) is a marginal nicety the agent largely compensates for by reading files itself, and it has a cheap escape hatch (§8.4).

### 8.1 The embedded terminal pane
A terminal pane is a window leaf whose content is a PTY plus a terminal grid.
- `portable-pty` spawns the shell or agent in a pseudo-terminal.
- Bytes from the PTY are parsed into a cell grid. Use `alacritty_terminal` as the reusable terminal core rather than hand-writing the escape-sequence layer — it gives you the parser and grid; you render its grid with the same diffed paint you use for buffers.
- When the pane is focused, keystrokes are written to the PTY; on resize, the PTY is told the new size.
- **Scope discipline (minimalist):** this terminal needs to be "good enough to run a shell and an agent," not a daily-driver xterm. Skip the long tail of obscure sequences. That keeps it a few days of work, not weeks.

### 8.2 Reflecting the agent's edits (the live-update experience)
The agent edits files **on disk**; file windows showing those files reflect the change. This is a manual-reload-first design, deliberately:

`revert-buffer`, bound to **`C-x C-r`** (avoid `C-r`, which is reverse isearch):
1. Re-read the file from disk into a fresh `ropey` rope.
2. Swap it into the buffer.
3. Reconcile the cursor — ship the lazy version first:
   - *Lazy:* clamp point to the same (line, col), or to EOF if the file shrank.
   - *Better:* remember a byte offset and clamp to new length.
   - *Nice, later:* diff old vs new (`similar`) and map point through it.
4. **Dirty-buffer guard:** if the buffer has unsaved edits *and* the disk changed, prompt before clobbering. One `if`, prevents a bad day.

**Optional auto path (later):** a `notify` watcher that does not auto-reload but lights a modeline marker (`[disk-changed]`) so you know a reload is available — you press the key when you see the agent finish in its pane. Auto-trigger is a further opt-in beyond that. The terminal pane actually makes this nicer: you can *see* the agent working, so you know exactly when to pull in changes.

### 8.3 Showing edits as diffs (optional polish)
When a file reloads, optionally render the delta rather than snapping: highlight added/removed lines, keep a "last seen" version (`similar`). Turns "the file blinked" into "I watched that change land." Nice, not load-bearing.

### 8.4 The cheap context escape hatch
The only thing the terminal-pane approach lacks vs a protocol is the editor *pushing* your cursor/selection to the agent. Recover most of it with one tiny command: `send-region-to-terminal` (or copy "path + line range + selection" to the clipboard) that types the current context into the agent's pane. A handful of lines, no subsystem. This is the deliberate 90/10 trade.

### 8.5 MCP — explicitly deferred, not a pillar
A future option, not part of v1, and no longer the architecture. If deep bidirectional awareness (editor pushes live cursor/selection/diagnostics to the agent as callable tools) ever proves worth it, Cortex could run an MCP server then. Until there's concrete evidence the terminal pane is insufficient, building it would violate §1a ("subtract first"). For current Claude Code capabilities (including MCP and hooks), see docs.claude.com/en/docs/claude-code/overview.

---

## 9. Aesthetics and Theming

- **Truecolor themes** shipped as defaults: Catppuccin, Gruvbox, Nord.
- **Modeline** at the bottom — study doom-modeline for the look. Shows buffer name, dirty state, position, active mode/markers (including `[disk-changed]`).
- **Diffed, flicker-free rendering** (see §6.2).
- Considered **padding**, **cursor styles** (block/bar), and tasteful use of color.
- Rule of thumb: truecolor + tree-sitter + a tasteful modeline is ~90% of why something reads as "modern" in a terminal.

---

## 10. Configuration

Cortex is configured by a **static, declarative file** (`config.toml` via `serde`), not a script. This is a deliberate simplification over Emacs: you get to rebind keys, pick a theme, and set options, but you do not get a live programmable runtime. The tradeoff buys a smaller, faster, more predictable editor and removes an entire embedding subsystem.

What the config covers:
- **Keybindings** — remap any key/chord sequence to any registered command by name (the command registry from §6.4 is what makes this possible without scripting).
- **Theme** — select a shipped theme or define truecolor overrides.
- **Settings** — padding, cursor style, soft-wrap, tab width, and default splits/tab behavior.

Principle (the Ghostty model): sane curated defaults so it's great with an empty config, but everything above is overridable. Anything that would require *logic* (conditionals, custom commands) is intentionally out of scope for v1 — if that itch becomes real, it's a deliberate future decision, not an accident.

---

## 11. Default Keybindings (Emacs-flavored)

| Binding | Command | Notes |
|---|---|---|
| `C-x C-s` | `save-buffer` | |
| `C-x C-c` | `quit` | prompts if dirty |
| `C-x C-f` | `find-file` | minibuffer + `nucleo` |
| `C-x b` | `switch-buffer` | fuzzy buffer switch |
| `C-x C-r` | `revert-buffer` | manual reload (§8.3) |
| `C-n` / `C-p` | next / previous line | |
| `C-f` / `C-b` | forward / backward char | |
| `C-a` / `C-e` | line start / end | |
| `C-Space` | `set-mark` | starts region |
| `C-k` | `kill-line` | onto kill ring |
| `C-y` | `yank` | from kill ring |
| `M-y` | `yank-pop` | cycle kill ring (after yank) |
| `C-s` | `isearch-forward` | incremental |
| `C-r` | `isearch-backward` | (reserved — not reload) |
| `C-x 2` | `split-window-below` | horizontal divider, stacked |
| `C-x 3` | `split-window-right` | vertical divider, side by side |
| `C-x 0` | `delete-window` | close focused split |
| `C-x 1` | `delete-other-windows` | keep only focused split |
| `C-x o` | `other-window` | move focus to next split |
| `C-x t 2` | `tab-new` | new tab (fresh layout) |
| `C-x t o` | `tab-next` | switch to next tab |
| `C-x t 0` | `tab-close` | close current tab |
| `C-c t` | `open-terminal-pane` | shell/agent in a split (§8.1) |
| `C-c s` | `send-region-to-terminal` | push path+selection to agent pane (§8.4) |
| `M-x` | `execute-command` | by name, via minibuffer |

---

## 12. Decisions Made / To Make

**Decided:**
- Name: **Cortex**.
- Modeless, Emacs-flavored (not Vim-modal).
- No LSP; AI is the intelligence layer; tree-sitter stays for highlighting.
- **No scripting runtime.** Declarative `config.toml` only; the introspectable command registry handles `M-x` and remapping. (Removes the former `mlua` vs `steel` decision entirely.)
- **In-editor splits + tabs** (§6.9), via an owned layout tree — not deferred to tmux.
- `ropey` for the buffer; per-window point/scroll; hand-rolled diffed rendering on `crossterm`.
- Manual reload (`C-x C-r`) first; `notify`/modeline-marker path later.
- **AI integration = a terminal pane running the agent** (§8). No MCP, no protocol. The window system is the integration surface.

**Open (decide during build):**
- Modeline: one global (focused window) vs per-window.
- Undo model: simple stack vs Emacs-style undo tree.
- Clipboard bridge: how tightly to couple the kill ring to the system clipboard.
- Terminal pane: start with `alacritty_terminal` unless it proves too heavy during implementation.

**Deferred (not in v1, not the architecture):**
- MCP server for bidirectional cursor/selection/diagnostics awareness — build only if the terminal pane proves insufficient.
- Inline AI primitives (region→model streamed into a buffer, ghost-text). The terminal pane covers the core workflow; these are additive if wanted later.

---

## 13. Roadmap

The issue-ready roadmap lives in `docs/roadmap.md`.
That file is the implementation tracking document.
This section remains the product-level phase overview.

**Phase 1 — Editing core (~1 week).**
Raw-mode loop, `ropey` buffer, diffed renderer, prefix-key trie, command dispatch, kill ring, minibuffer, point/mark, isearch. Outcome: a real editor you can open, edit, search, and save in.

**Phase 2 — Windows + aesthetics (~1 week).**
Window layout tree (splits), tab bar, focus/navigation commands, per-window viewports; then tree-sitter highlighting, themes, modeline, padding/cursor polish. Outcome: a multi-pane editor that looks modern. (Splits land here because the renderer's rectangle handling and the modeline both depend on the window model.)

**Phase 3 — Workflow + the terminal pane (~1 week).**
`find-file`/fuzzy buffer switch, undo/redo, declarative `config.toml`, `revert-buffer` (manual reload + dirty guard), and the **embedded terminal pane** (`portable-pty` + `alacritty_terminal`) that runs a shell or Claude Code in a split. Outcome: daily-drivable, and you can run the agent beside your code and reload its edits. This is the whole AI story for v1.

**Phase 4 — Polish (post-v1, optional).**
`notify` `[disk-changed]` marker, diff-rendered reloads, `send-region-to-terminal` context helper. Each is small and independent — add when it itches.

Net: genuinely daily-drivable in **roughly a month** of focused work. The terminal pane is the only meaningfully new subsystem beyond a plain editor, and reusing the window system + cell grid keeps even that contained.

---

## 14. Reference Implementations to Study

- **`mg` (micro GNU Emacs, C)** — the canonical "minimum viable Emacs"; a scope guide for what to include vs skip.
- **`hecto` / `kibi` (Rust)** — tiny kilo-style editors; the raw-mode + render-loop primer.
- **Helix (Rust)** — how a modern Rust terminal editor structures rendering, tree-sitter, and (its) scripting via `steel`. Ignore its modal model and LSP parts.
- **`amp` (Rust)** — another Rust terminal editor for structural reference.
- **`alacritty_terminal` / Rio (Rust)** — the reusable terminal core and a wgpu terminal; study these for the embedded terminal pane (§8.1).
- **Lem (Common Lisp)** — a from-scratch Emacs-like terminal editor; useful for its window/buffer model and overall Emacs-feel-in-a-modern-codebase, even though Cortex skips the Lisp-everywhere extensibility.
- **doom-modeline** — the modeline look to emulate.

---

*End of spec v0.3.*

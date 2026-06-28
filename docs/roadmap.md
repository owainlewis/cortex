# Cortex Roadmap

This roadmap shows what is done, what is planned, and which GitHub issues track the work.
It is not a changelog.
It is not the detailed implementation plan.

Use GitHub issues for acceptance criteria and task detail.
Use merged PRs, direct commits, and linked issues as proof of shipped behavior.
Use this file for product direction and status.

Status meanings:
- Done: the tracking issues are closed.
- Planned: the tracking issues are open.
- Not ticketed: the phase needs issues before work starts.

## Direction

Cortex is a macOS-only terminal code editor.
It is built for the author's own workflow first.
The goal is a small, fast editor with Emacs-style keys and a terminal-native agent workflow.

Cortex should stay simple.
It should prefer one clear way to do a thing.
It should not add Windows, Linux, LSP, plugins, scripting, or broad configuration until the roadmap explicitly calls for them.

## v0.1 Minimal Editor

Status: Done.

Goal: open one file, edit it, save it, and quit cleanly.

Includes:
- Rust binary crate.
- Terminal raw mode and cleanup.
- Rope-backed text buffer.
- Basic rendering.
- Cursor movement.
- Text editing.
- Save behavior.
- Dirty quit prompt.
- README smoke notes.

Not included:
- Directory picker.
- Slash commands.
- Search.
- Syntax highlighting.
- Multiple buffers.
- Splits or tabs.
- Config.
- LSP.
- Agent workflow.

Tracking issues:
- [closed] [#1 Scaffold the Rust project](https://github.com/owainlewis/cortex/issues/1)
- [closed] [#2 Add terminal lifecycle and app shell](https://github.com/owainlewis/cortex/issues/2)
- [closed] [#3 Implement rope-backed buffer and file behavior](https://github.com/owainlewis/cortex/issues/3)
- [closed] [#4 Implement cursor and viewport movement](https://github.com/owainlewis/cortex/issues/4)
- [closed] [#5 Render the single-file editor](https://github.com/owainlewis/cortex/issues/5)
- [closed] [#6 Add input, keymap prefix handling, and command dispatch](https://github.com/owainlewis/cortex/issues/6)
- [closed] [#7 Add save, status messages, and dirty quit prompt](https://github.com/owainlewis/cortex/issues/7)
- [closed] [#8 Manual smoke test and README notes](https://github.com/owainlewis/cortex/issues/8)

Release notes should focus on the single-file editing loop and terminal cleanup.

## v0.2 Editor Surface

Status: Done.

Goal: make the one-buffer editor easier to use.

Includes:
- Directory startup.
- File picker.
- Slash command line.
- Visual theme.
- Modeline cleanup.
- First syntax highlighting support.
- README updates.

Not included:
- Multiple buffers.
- Splits or tabs.
- Kill ring.
- Config.
- Plugins.
- LSP.
- Embedded agent pane.

Tracking issues:
- [closed] [#17 Open directories with a minimal file picker](https://github.com/owainlewis/cortex/issues/17)
- [closed] [#18 Add slash command line for named editor commands](https://github.com/owainlewis/cortex/issues/18)
- [closed] [#19 Add visual theme and modeline polish](https://github.com/owainlewis/cortex/issues/19)
- [closed] [#20 Add tree-sitter syntax highlighting](https://github.com/owainlewis/cortex/issues/20)
- [closed] [#21 Update docs for v0.2 behavior](https://github.com/owainlewis/cortex/issues/21)

Release notes should focus on opening files, command discovery, visual clarity, and highlighting.

## v0.3 Editing

Status: Planned.

Goal: make everyday editing safer and faster.

Likely includes:
- Undo and redo.
- Minibuffer foundation.
- Incremental search.
- Multiple buffers.
- Find file.
- Switch buffer.

Tracking issues:
- [open] [#27 Add undo and redo](https://github.com/owainlewis/cortex/issues/27)
- [open] [#28 Add minibuffer foundation](https://github.com/owainlewis/cortex/issues/28)
- [open] [#29 Add incremental search](https://github.com/owainlewis/cortex/issues/29)
- [open] [#30 Add multiple buffers, find-file, and switch-buffer](https://github.com/owainlewis/cortex/issues/30)

Release notes should focus on editing safety, search, and buffer navigation.

## v0.4 Windows

Status: Planned.

Goal: show and manage more than one working context.

Likely includes:
- Split layout tree.
- Tabs.
- Per-window view state.
- Focus movement.
- Modeline behavior for windows.

Tracking issues:
- [open] [#31 Add split window layout tree](https://github.com/owainlewis/cortex/issues/31)
- [open] [#32 Add tabs over window layouts](https://github.com/owainlewis/cortex/issues/32)

Release notes should focus on splits, tabs, and navigation.

## v0.5 Agent Workflow

Status: Not ticketed.

Goal: support terminal-based coding agents without adding an AI platform.

Likely includes:
- Manual file reload.
- Dirty reload guard.
- Disk changed indicator.
- Embedded terminal pane if the window model is ready.

Tracking issues:
- None yet.

Release notes should focus on reload safety and working with files changed by agents.

## Release and Install

Status: Done.

Goal: make Cortex buildable, installable, and releasable from GitHub.

Includes:
- Rust CI.
- Tag-based macOS release builds.
- Install script.
- Version and update command surface.
- Nightly build workflow.
- Release documentation.

Tracking issues:
- [closed] [#33 Add GitHub Actions CI for Rust checks](https://github.com/owainlewis/cortex/issues/33)
- [closed] [#34 Add tag-based macOS release builds](https://github.com/owainlewis/cortex/issues/34)
- [closed] [#35 Add install.sh for latest release installs](https://github.com/owainlewis/cortex/issues/35)
- [closed] [#36 Add version and update command surface](https://github.com/owainlewis/cortex/issues/36)
- [closed] [#37 Add nightly build workflow](https://github.com/owainlewis/cortex/issues/37)
- [closed] [#38 Document install, release, and update workflow](https://github.com/owainlewis/cortex/issues/38)

Release notes should focus on install, update, CI, and release workflow changes.

## Later Quality Work

Status: Not ticketed.

Goal: make the editor faster, clearer, and more durable.

Likely includes:
- Diffed rendering.
- Reload diff highlights.
- More syntax coverage.
- Better modeline details.
- Theme cleanup.
- Large-file performance work.

Tracking issues:
- None yet.

## Rules

Keep Cortex macOS-only unless the product direction changes.
Keep release notes grounded in PRs, commits, and linked issues.
Keep roadmap detail at the milestone level.
Put implementation detail in GitHub issues or `docs/issues/`.

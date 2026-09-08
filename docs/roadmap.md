# Cortex Roadmap

This roadmap shows what is done, what is planned, and which GitHub issues track the work.
It is not a changelog.
It is not the detailed implementation plan.

Use GitHub issues for acceptance criteria and task detail.
Use merged PRs, direct commits, and linked issues as proof of shipped behavior.
Use this file for product direction and status.

Status meanings:
- Done: the tracking issues are closed.
- In progress: some tracking issues are closed while related work remains open.
- Planned: the tracking issues are open.
- Not ticketed: the phase needs issues before work starts.

## Direction

Cortex is a macOS-only terminal code editor.
It is built for the author's own workflow first.
The goal is a small, fast editor with Emacs-style keys and a terminal-native agent workflow.
Cortex keeps multiple buffers and one visible editor view.
Use tmux or host-terminal panes for shells and coding agents.

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

Status: In progress.

Goal: make everyday editing safer and faster.

Shipped:
- Undo and redo.
- Minibuffer foundation.
- Multiple buffers.
- Find file.
- Switch buffer.

Planned:
- Incremental search.
- Command registry and `M-x`.
- Kill ring and yank-pop.

Tracking issues:
- [closed] [#27 Add undo and redo](https://github.com/owainlewis/cortex/issues/27)
- [closed] [#28 Add minibuffer foundation](https://github.com/owainlewis/cortex/issues/28)
- [open] [#29 Add incremental search](https://github.com/owainlewis/cortex/issues/29)
- [closed] [#30 Add multiple buffers, find-file, and switch-buffer](https://github.com/owainlewis/cortex/issues/30)
- [open] [#46 Add command registry and M-x](https://github.com/owainlewis/cortex/issues/46)
- [open] [#47 Replace cut slot with a real kill ring](https://github.com/owainlewis/cortex/issues/47)

Release notes should focus on editing safety, search, and buffer navigation.

## Daily coding delivery

Status: In progress.
Parent: [#160 Deliver a minimalist daily coding editor](https://github.com/owainlewis/cortex/issues/160).
This extends the v0.3 editing work; individual ticket criteria define completion.

Delivery order:
- [closed] [#161 Align the product direction](https://github.com/owainlewis/cortex/issues/161)
- [open] [#162 Correct TypeScript and fenced Markdown colours](https://github.com/owainlewis/cortex/issues/162)
- [open] [#46 Add command registry and M-x](https://github.com/owainlewis/cortex/issues/46)
- [open] [#164 Add typing and region indentation](https://github.com/owainlewis/cortex/issues/164)
- [open] [#165 Handle literal terminal paste](https://github.com/owainlewis/cortex/issues/165)
- [open] [#166 Group typing and deletion undo steps](https://github.com/owainlewis/cortex/issues/166)
- [open] [#47 Add kill ring and yank-pop](https://github.com/owainlewis/cortex/issues/47)
- [open] [#167 Add explicit macOS clipboard commands](https://github.com/owainlewis/cortex/issues/167)
- [open] [#168 Add word, page, and line navigation](https://github.com/owainlewis/cortex/issues/168)
- [open] [#29 Add incremental search](https://github.com/owainlewis/cortex/issues/29)
- [open] [#169 Add fuzzy file and buffer navigation](https://github.com/owainlewis/cortex/issues/169)
- [open] [#170 Add literal query-replace](https://github.com/owainlewis/cortex/issues/170)
- [open] [#163 Keep deep-file highlighting responsive](https://github.com/owainlewis/cortex/issues/163)
- [open] [#171 Refine the coding surface](https://github.com/owainlewis/cortex/issues/171)
- [open] [#172 Verify the full coding workflow](https://github.com/owainlewis/cortex/issues/172)

Dependencies are recorded in each issue.
The final workflow check must verify code, terminal behavior, performance, and documentation against every delivered task.
Publishing a release is separate from this implementation batch.

## External terminal workflow

Manual reload, the dirty reload guard, and the disk-changed indicator are implemented through [#48](https://github.com/owainlewis/cortex/issues/48).
Idle disk-change notification and a verified tmux workflow are included in #172.
Internal split layouts (#31), tabs (#32), and an embedded terminal pane (#49) are superseded by the external-terminal direction in #161.
Those tickets are closed as not planned and retain their original design history.
Revisit internal views only for a demonstrated need to show the same unsaved buffer in two places.

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

## Editor Quality

Status: In progress.

Goal: keep editing correct, safe, and fast as the editor grows.

Likely includes:
- Diffed rendering.
- Latency and large-file performance checks.
- File-save hardening.
- Rendering and Unicode correctness.

Tracking issues:
- [closed] [#50 Add diffed rendering](https://github.com/owainlewis/cortex/issues/50)
- [closed] [#51 Add latency and large-file performance checks](https://github.com/owainlewis/cortex/issues/51)
- [closed] [#58 Allow literal slash insertion without losing command entry](https://github.com/owainlewis/cortex/issues/58)
- [closed] [#59 Harden atomic-save temporary file creation](https://github.com/owainlewis/cortex/issues/59)
- [closed] [#60 Preserve file permissions and define symlink save behavior](https://github.com/owainlewis/cortex/issues/60)
- [closed] [#61 Remove whole-buffer work from input and render paths](https://github.com/owainlewis/cortex/issues/61)
- [closed] [#62 Preserve syntax highlight context across viewport boundaries](https://github.com/owainlewis/cortex/issues/62)
- [closed] [#63 Render Unicode grapheme clusters and combining marks correctly](https://github.com/owainlewis/cortex/issues/63)
- [closed] [#64 Mark final-newline-only changes in the gutter](https://github.com/owainlewis/cortex/issues/64)
- [closed] [#110 Reconcile README and roadmap after final audit](https://github.com/owainlewis/cortex/issues/110)
- [closed] [#111 Restore terminal state on catchable termination signals](https://github.com/owainlewis/cortex/issues/111)
- [closed] [#112 Bound highlighting work at deep viewports](https://github.com/owainlewis/cortex/issues/112)
- [closed] [#113 Keep point visible while editing long lines](https://github.com/owainlewis/cortex/issues/113)
- [closed] [#114 Avoid full long-line comparisons during rendering](https://github.com/owainlewis/cortex/issues/114)

## Repository Quality

Status: Done.

Goal: keep project tracking, contribution flow, CI, security, and releases dependable.

Tracking issues:
- [closed] [#52 Reconcile README, roadmap, and issue state with shipped behavior](https://github.com/owainlewis/cortex/issues/52)
- [closed] [#65 Protect main with required Rust checks](https://github.com/owainlewis/cortex/issues/65)
- [closed] [#66 Create active roadmap tracking with milestones and a Cortex Project](https://github.com/owainlewis/cortex/issues/66)
- [closed] [#67 Enable dependency vulnerability scanning and update automation](https://github.com/owainlewis/cortex/issues/67)
- [closed] [#68 Add the repository labels required by STANDARDS.md](https://github.com/owainlewis/cortex/issues/68)
- [closed] [#69 Gate tagged releases on formatting, clippy, tests, and build](https://github.com/owainlewis/cortex/issues/69)
- [closed] [#70 Pin GitHub Actions and add release provenance](https://github.com/owainlewis/cortex/issues/70)
- [closed] [#71 Standardize merge strategy and clean stale branches](https://github.com/owainlewis/cortex/issues/71)
- [closed] [#72 Add contributing, security, and issue-reporting guidance](https://github.com/owainlewis/cortex/issues/72)
- [closed] [#73 Make the macOS arm64 build target deterministic](https://github.com/owainlewis/cortex/issues/73)

## Rules

Keep Cortex macOS-only unless the product direction changes.
Keep release notes grounded in PRs, commits, and linked issues.
Keep roadmap detail at the milestone level.
Put implementation detail in GitHub issues or `docs/issues/`.

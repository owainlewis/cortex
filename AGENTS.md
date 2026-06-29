# Cortex Agent Instructions

These instructions apply to this repository.
Global instructions still apply unless this file is more specific.

## Product Direction

- Cortex is a macOS-only terminal code editor.
- Build for the author's macOS workflow first.
- Do not add Windows or Linux compatibility work unless explicitly requested.
- The product ambition is the best fast lightweight code editor in the world for this workflow.
- Think Emacs power without Emacs bloat.
- Keep the editor small, coherent, beautiful, and fast.
- Prefer one clear way to do a thing over options and layers.
- Take pride in details: latency, terminal cleanup, cursor feel, spacing, typography assumptions, modeline polish, and error states.

## Engineering Defaults

- Make the smallest complete change that satisfies the active issue.
- Keep behavior aligned with `docs/prd.md` and `docs/roadmap.md`.
- Use `ropey` for text storage unless the PRD changes.
- Keep point and scroll in view/editor state, not only in the buffer.
- Keep rendering behind a renderer boundary so diffed rendering can replace simple redraw later.
- Do not add LSP, plugins, scripting, config, splits, tabs, or AI integration before the roadmap asks for them.
- Add focused tests for pure editor logic whenever behavior changes.

## Workflow

- If working from a GitHub issue, read the issue, `docs/prd.md`, and `docs/roadmap.md` before editing.
- Keep GitHub Project status current when running the coordinator workflow in `docs/prompt.md`.
- Write an issue plan under `docs/issues/<issue-number>-plan.md` only when the implementation is non-trivial.
- Run the relevant verification from the issue before reporting completion.
- For terminal-facing changes, manually check that the shell is usable after exit.

## Writing

- Keep Markdown plain and practical.
- When writing or substantially editing long Markdown files, put each full sentence on its own physical line.
- Avoid hype.
- Use concrete claims about behavior, scope, and verification.

# v0.1.0 Release Notes Goal Prompt

Use this prompt in Codex when preparing or demoing the Cortex v0.1.0 release notes flow.

It is intentionally a manual release-prep hook, not a git hook.

Run it before tagging or publishing a GitHub release.

```md
/goal A source-linked release notes draft exists for Cortex v0.1.0, grounded only in merged PRs, direct commits, and linked issues.
Stop when the draft is written or when the available source material is too ambiguous for one safe draft.

You are drafting release notes for the first Cortex release, `v0.1.0`.

Read these files first for scope context only:
- `docs/prd.md`
- `docs/roadmap.md`
- `README.md`
- `Cargo.toml`

Do not use `docs/prd.md`, `docs/roadmap.md`, or `README.md` as proof that a feature shipped.
Use them only to understand intended v0.1.0 scope, current user-facing language, and version metadata.
Do not cite those files as source links for release note bullets unless the bullet is specifically about documentation.

Define the scan window:
- If a `v0.1.0` tag already exists, compare the tag target with its parent history and use the commits that led to that tag.
- If no release tag exists, treat this as a first-release draft and scan from the initial commit to `HEAD`.
- If this includes work beyond v0.1.0, use `docs/roadmap.md` to keep only the v0.1 minimal-editor scope and flag later work for human review.

Find source material:
- List merged PRs in the scan window when GitHub metadata is available.
- List direct commits in the scan window.
- Read linked issues only when they are referenced by a merged PR or commit.
- Do not use unmerged branches, unreleased plans, or guesses.
- Treat roadmap and PRD claims as plans unless a merged PR, commit, or linked issue confirms the behavior landed.
- Use commits, merged PRs, or linked issues as source links for shipped behavior.

Draft `docs/releases/v0.1.0-draft.md` with this structure:

# Cortex v0.1.0 Release Notes Draft

## Summary

A short plain-English summary of what v0.1.0 gives the user.

## User-Facing Changes

Bullets for visible editor behavior, each with a PR, commit, or issue link.

## Fixes

Bullets for bug fixes, each with a PR, commit, or issue link.

## Docs

Bullets for documentation changes, each with a PR, commit, or issue link.

## Internal Changes

Bullets for implementation or project-structure changes, each with a PR, commit, or issue link.

## Human Review

List possible breaking changes, security notes, unclear user impact, source gaps, and any work that appears to belong to a later milestone.

Rules:
- Do not publish a GitHub release.
- Do not create or move git tags.
- Do not edit `CHANGELOG.md` or create one.
- Do not edit the live README.
- Do not invent features, fixes, or user impact.
- Prefer `Needs review` over speculation.
- Keep Cortex's scope clear as a macOS-only terminal editor.

Success:
- `docs/releases/v0.1.0-draft.md` exists.
- Every release note has at least one source link.
- Ambiguous or later-milestone work is clearly flagged for human review.
```

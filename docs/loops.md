# Codex Goal Loops

This document lists high-level `/goal` prompts for keeping Cortex healthy.
The point is to move from manual prompts to repo-level instructions.

Each loop should discover what needs doing, make safe fixes, and stop only when the repo is cleaner or a human decision is needed.
The long-term goal is a codebase that keeps its roadmap, issues, docs, tests, CI, and releases in sync with minimal manual prompting.

## Repo Maintenance

What this loop is: regular repo health management.

Description: Use this as the default maintenance loop.
It checks docs, roadmap status, backlog health, tests, CI, dependencies, and release readiness.
It fixes safe problems and writes down anything that needs a decision.

Prompt to run:

```md
/goal Cortex has completed a full repo maintenance pass, with safe fixes applied and remaining decisions clearly documented.
Stop when the maintenance report is written and all safe fixes are complete, or when a human decision blocks progress.

Read `README.md`, `docs/prd.md`, `docs/roadmap.md`, `docs/loops.md`, open GitHub issues, recent merged PRs, current GitHub Actions status, `Cargo.toml`, and the Rust test suite.

Check:
- README matches current user-facing behavior.
- PRD is still product direction, not shipped-feature evidence.
- Roadmap status matches GitHub issues and merged work.
- Open issues are actionable or clearly parked.
- Tests cover the most important pure editor behavior.
- CI and release workflows match the commands developers actually run.
- Dependencies look intentional and in scope.

Apply safe fixes:
- Update stale docs when the correct wording is clear.
- Update roadmap ticket links and status from GitHub issue state.
- Add or adjust focused tests for clear pure-logic gaps.
- Fix small CI, docs, or dependency issues when intent is obvious.
- Comment on or close only issues that are clearly stale, duplicate, or obsolete.

Write `docs/repo-maintenance.md` with:
- Date
- Checks run
- Fixes made
- Issues updated or closed
- Tests run
- CI status
- Release readiness notes
- Human decisions needed

Do not make product direction changes without approval.
Do not publish releases.
Do not create tags.
Do not batch risky implementation work into the maintenance pass.

Success means the repo is easier to trust and the remaining work is explicit.
```

## Roadmap Update

What this loop is: keep the roadmap accurate.

Description: Use this when the roadmap may be stale.
It compares roadmap phases with GitHub issues, merged PRs, docs, and current code.
It updates the roadmap at the milestone level and creates or recommends missing tickets.

Prompt to run:

```md
/goal `docs/roadmap.md` accurately shows what is done, what is planned, what is not ticketed, and which GitHub issues track each phase.
Stop when the roadmap is updated or when product direction needs human approval.

Read `docs/prd.md`, `docs/roadmap.md`, `README.md`, open and closed GitHub issues, recently merged PRs, and current source layout.

Update `docs/roadmap.md` so it stays high level:
- Keep simple phase names.
- Show status as Done, Planned, or Not ticketed.
- Link tracking issues for each phase.
- Keep includes and not-included lists short.
- Keep release-note guidance grounded in shipped work.
- Remove detailed acceptance criteria and implementation plans.

Create or recommend GitHub issues when:
- A roadmap phase says Not ticketed.
- A planned phase is too broad to work safely.
- Current code has moved ahead of the roadmap.
- A done phase has missing tracking links.

Do not treat PRD, README, or roadmap claims as proof that features shipped.
Use merged PRs, direct commits, linked issues, and current code for factual updates.
Do not change product direction without approval.

Success means a reader can answer:
- What is done?
- What is next?
- What tickets track it?
- What still needs product judgment?
```

## Backlog Management

What this loop is: keep issues useful.

Description: Use this when the issue list is noisy or stale.
It reviews the backlog, closes obvious junk, and leaves the remaining issues actionable.

Prompt to run:

```md
/goal The Cortex GitHub backlog is useful: every open issue is actionable, intentionally parked, or waiting on a clear decision.
Stop when every open issue has been reviewed or when product judgment is required.

Read `docs/prd.md`, `docs/roadmap.md`, and all open GitHub issues for `owainlewis/cortex`.

For each issue, decide whether it is:
- Actionable now.
- Planned for a roadmap phase.
- Blocked.
- Duplicate.
- Stale.
- Out of scope.
- Unclear.

Apply safe updates:
- Close duplicates and obsolete issues with a short comment.
- Add clarifying comments to blocked or unclear issues.
- Recommend labels or milestone placement when useful.
- Create follow-up issues only when the missing work is clear and not already tracked.

Write `docs/backlog-management.md` with issue counts, actions taken, open questions, and recommended next issues to work.

Do not close issues that require product judgment.
Do not add broad speculative issues.

Success means the backlog can drive work without another manual cleanup pass.
```

## Milestone Management

What this loop is: manage a roadmap phase from tickets to merged work.

Description: Use this for a larger demo.
It works through one roadmap phase, one ticket at a time, and keeps status current.

Prompt to run:

```md
/goal The [NEEDS: milestone] Cortex milestone is complete according to its tracking tickets and merged PRs.
Stop when every ticket in the milestone is merged, or when the next ticket is blocked by product direction, failing auth, failing checks, or unclear acceptance criteria.

Read `docs/prd.md`, `docs/roadmap.md`, and every tracking issue for the milestone.
Confirm the issue order and dependency order.

For each ticket:
- Confirm it still fits the roadmap.
- Create a focused branch.
- Make the smallest complete implementation.
- Add or update focused tests for changed behavior.
- Run the required verification and `cargo test`.
- Open a PR with issue link, summary, tests, and known limitations.
- Fix failing checks.
- Merge only when the PR is ready.
- Update issue and roadmap status as needed.

Do not batch unrelated tickets into one PR.
Do not expand the milestone while delivering it.
Do not change product direction without approval.

Success means the milestone is done or the remaining blocker is clearly documented.
```

## Quality Management

What this loop is: keep behavior tested and reliable.

Description: Use this when code quality matters more than shipping new features.
It reviews tests, finds real bugs, runs smoke checks where practical, and records gaps.

Prompt to run:

```md
/goal Cortex has completed a quality pass, with confirmed bugs fixed, important test gaps addressed, and remaining risks documented.
Stop when the quality report is written and the first safe batch of fixes passes.

Read `README.md`, `docs/prd.md`, `docs/roadmap.md`, current source, and tests.

Review:
- Buffer editing.
- Save safety.
- Dirty state.
- Cursor movement.
- Key dispatch.
- Slash commands.
- Picker behavior.
- Syntax highlighting.
- Rendering bounds.
- Terminal cleanup.
- Filesystem errors.

For each suspected issue, prove it with a test, reproduction, or clear code path.
Fix only confirmed bugs with clear expected behavior.
Add focused pure-logic tests where they will prevent regressions.
Run `cargo test`.

Write `docs/quality-management.md` with:
- Tests reviewed
- Tests added
- Bugs fixed
- Smoke checks run
- Risks left open
- Follow-up issues recommended

Do not refactor unrelated code.
Do not fix speculative issues without proof.

Success means the repo is safer than before and the remaining risks are visible.
```

## Release Management

What this loop is: prepare and verify a release.

Description: Use this before tagging or publishing.
It drafts release notes, checks readiness, verifies CI and install docs, and stops before release actions that need approval.

Prompt to run:

```md
/goal Cortex is ready for release [NEEDS: version], with source-linked release notes and a readiness report.
Stop when the release is ready for human approval or when a named blocker prevents release.

Inputs:
- Target version: [NEEDS: version]
- Base tag: [NEEDS: previous release tag or first-release range]

Read `docs/prd.md`, `docs/roadmap.md`, `README.md`, `Cargo.toml`, release workflow files, open issues, merged PRs, direct commits, and linked issues.

Draft `docs/releases/[version]-draft.md`:
- Use merged PRs, direct commits, and linked issues as sources.
- Do not use PRD, roadmap, or README as proof that behavior shipped.
- Group changes into user-facing changes, fixes, performance, docs, internal changes, breaking changes, and security notes.
- Link every shipped claim to a source.
- Flag unclear impact for review.

Write `docs/releases/[version]-readiness.md`:
- Version metadata
- Local checks run
- GitHub Actions status
- Release workflow status
- Install and update docs status
- Open blockers
- Exact next release steps

Run relevant local checks, including `cargo test`.

Do not create tags.
Do not publish a GitHub release.
Do not edit a live changelog unless explicitly asked.

Success means a human can approve or block the release from the reports.
```

## CI and Install Management

What this loop is: keep build, install, and release automation healthy.

Description: Use this when CI, nightly builds, release artifacts, install scripts, or update commands need review.
It checks the automation as a system instead of treating each workflow as a separate task.

Prompt to run:

```md
/goal Cortex CI, install, and release automation are consistent, tested where possible, and documented.
Stop when automation is green and documented, or when a permission, secret, or product decision blocks progress.

Read GitHub Actions workflows, install scripts, update command implementation and docs, `Cargo.toml`, `README.md`, and recent workflow runs.

Check:
- CI runs the expected Rust checks.
- Local commands match CI commands.
- Release workflow triggers from the expected tags.
- Artifact names match install and update docs.
- Nightly builds are understandable and not confused with releases.
- Version output and update surfaces match docs.

Fix safe mismatches.
Run local checks.
Inspect recent GitHub Actions runs.
Write `docs/ci-install-management.md` with checks run, fixes made, current status, and remaining risks.

Do not remove checks to make CI pass.
Do not change secrets, signing, or publishing permissions.
Do not create tags or publish releases.

Success means automation is understandable and safe to rely on.
```

## Demo Prep

What this loop is: prepare the repo for a Codex `/goal` demo.

Description: Use this before recording or presenting.
It chooses the best goal loops to show and checks that the repo state will not confuse the audience.

Prompt to run:

```md
/goal The Cortex repo is ready for a live Codex `/goal` demo.
Stop when the demo plan is written or when a blocker needs human choice.

Read `docs/loops.md`, `docs/roadmap.md`, `docs/prd.md`, `README.md`, GitHub issue state, current branch, and worktree status.

Pick three demo options:
- A short safe loop.
- A medium useful loop.
- A larger ambitious loop.

For each option, explain:
- What it demonstrates.
- What artifact it will produce.
- How long it may take.
- What could go wrong.
- What the fallback is.

Check that roadmap status, issue state, release docs, and worktree status are explainable.
Apply small documentation fixes only when they make the demo more truthful.
Write `docs/demo-prep.md`.

Do not hide real repo state.
Do not rewrite the repo just for presentation polish.

Success means the demo has a clear plan and safe fallback options.
```

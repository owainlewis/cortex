# Cortex Standards

These standards define what healthy means for this repository.
Factory and other agents should use this file when reviewing the repo, opening issues, and preparing PRs.

## Required Standards

### License

- The repo must have a `LICENSE` file.
- The license must be MIT unless the owner explicitly changes it.
- `Cargo.toml` and `README.md` must name the same license.

Factory may open a PR to add a missing MIT license file when the repo clearly says MIT elsewhere.
Factory must stop and ask if the intended license is unclear or if the work would change the license.

### Docs Match Code

- `README.md` must describe behavior that exists in the current code or in shipped releases.
- `docs/prd.md` must describe product intent, not claim that planned features have shipped.
- `docs/roadmap.md` must match GitHub issue state for planned and completed work.
- Release docs must be based on merged PRs, commits, tags, and linked issues.
- Keybindings, slash commands, install commands, and known limitations must match the code.

Factory may open PRs for small doc corrections.
Factory must open an issue instead of changing docs when the product decision is unclear.

### GitHub Project

- Active milestone work must have a GitHub Project.
- Issues selected for active milestone work must be present on that project.
- Project status must reflect real work state.
- Done means the linked PR or direct commit is merged.
- In Progress means active implementation, review, checks, or blocked work after start.
- Todo means planned work that has not started.

Factory may add missing issue links to the project when GitHub permissions allow.
Factory must not create or rename project fields without human approval.

### GitHub Issues And Labels

Every open issue must have at least one type label:

- `bug`: broken behavior.
- `documentation`: docs are missing, stale, or wrong.
- `enhancement`: user-visible feature work.
- `quality`: tests, performance, refactors, CI, or internal code health.

Factory-specific labels:

- `factory-ready`: an agent may work this issue without more product input.
- `factory-triage`: the issue needs clarification, acceptance criteria, or scope shaping.
- `factory-needs-human`: the issue needs a human decision before implementation.
- `factory-blocked`: the issue cannot move until a named blocker is resolved.

An issue may have `factory-ready` only when all of these are true:

- The expected behavior is clear.
- The acceptance criteria are clear.
- The work is small enough for one focused PR.
- The issue does not require a human review item listed below.
- The issue has no known blocker.

Use `factory-needs-human` for:

- product direction changes
- license changes
- release decisions
- public claims
- pricing or business decisions
- deleting features
- broad dependency or architecture choices
- unclear editor behavior or keybinding decisions

Factory may add missing labels when the right label is clear.
Factory must comment and stop when the label choice depends on judgment.

### Tests

- Behavior changes must include focused tests when the behavior can be tested without brittle terminal mocks.
- Pure editor logic must have unit tests.
- File behavior must test save, load, missing file, and error paths when touched.
- Keybinding and command dispatch changes must test the changed binding or command.
- Terminal-facing changes must include manual smoke evidence when automated tests are not enough.

Required local checks:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Factory must run the relevant checks before opening a PR.
Factory must explain any check it could not run.

### CI

- CI must run on pull requests.
- CI must run on pushes to `main`.
- CI must run formatting, clippy, tests, and a release build.
- CI must use the stable Rust toolchain unless the repo explicitly changes toolchains.
- CI must not require secrets for normal pull request checks.

Factory may open PRs for small CI fixes.
Factory must stop and ask before changing release, signing, publishing, or secret-related workflows.

### Agent Instructions

- `AGENTS.md` must exist.
- Agent instructions must say Cortex is macOS-only.
- Agent instructions must require small scoped changes.
- Agent instructions must require docs to stay aligned with `docs/prd.md` and `docs/roadmap.md`.
- Agent instructions must block broad product changes without human review.

Factory may open PRs for small instruction fixes.
Factory must not weaken safety rules.

## Recommended Standards

- `CONTRIBUTING.md` explains local setup, checks, issue flow, and PR expectations.
- `SECURITY.md` explains how to report security issues.
- Open issues link to roadmap phases when they are roadmap work.
- Non-trivial issues have a plan under `docs/issues/<issue-number>-plan.md`.
- Performance-sensitive work includes a benchmark, smoke check, or clear manual test.

Factory may open issues for missing recommended standards.
Factory should not block normal work on recommended standards.

## Human Review Required

Human review is required before:

- merging PRs
- cutting releases
- changing the license
- changing product direction
- changing editor behavior that is not clearly requested by an issue
- changing keybindings that are not clearly requested by an issue
- adding large dependencies
- deleting features
- making public claims
- changing safety rules

Factory must stop and ask when work touches these areas and the issue does not already give clear approval.

## Factory Review Rule

When Factory reviews this repo against these standards, it should classify each failed standard as one of:

- `fix`: open the smallest safe PR.
- `issue`: open or update a focused issue.
- `blocked`: report the missing human decision or permission.

Factory should prefer PRs for mechanical fixes.
Factory should prefer issues for judgment calls.
Factory must not merge.
Factory must not push to `main`.

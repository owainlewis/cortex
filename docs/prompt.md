Create a Codex goal for this thread:

Work through GitHub issues #1-#8 in owainlewis/cortex and turn them into merged PRs, one issue at a time, using a coordinator workflow.

Repository:
- Local path: /Users/owainlewis/Code/github/owainlewis/cortex
- GitHub repo: owainlewis/cortex
- Remote: git@github.com:owainlewis/cortex.git
- Base branch: main

Project tracking:
- GitHub Project: Cortex v0.1
- Project URL: https://github.com/users/owainlewis/projects/13
- Project number: 13
- Project owner: owainlewis
- Status lanes: Todo, In Progress, Done

Source of truth:
- docs/prd.md
- docs/roadmap.md
- GitHub issues #1-#8
- GitHub Project #13

Codex thread policy:
- The coordinator thread coordinates.
- Implementation work must happen in separate Codex app threads, not internal subagents.
- Do not implement issues directly in the coordinator thread unless `codex_app.create_thread` is unavailable or fails.
- Before creating a worker thread, call `codex_app.list_projects` and find the project for `/Users/owainlewis/Code/github/owainlewis/cortex`.
- For each active issue lane, call `codex_app.create_thread` with a project target and a worktree environment.
- Prefer a worktree starting from `main` for each issue worker.
- Give each worker a self-contained prompt with the issue, repo context, branch name, acceptance criteria, verification steps, project status rules, PR expectations, and reporting requirements.
- Use internal subagents only for adversarial review inside a worker thread.
- Do not use an internal subagent as a substitute for an implementation worker thread.

Project status rules:
- Keep the project board current as work progresses.
- Before starting each issue, confirm the project item exists on GitHub Project #13.
- When you start active work on an issue, move its project status to `In Progress`.
- Leave the issue in `In Progress` while planning, coding, reviewing, fixing, opening the PR, and waiting for checks.
- Move the issue to `Done` only after the PR has been merged into `main`.
- If an issue becomes blocked, leave it in `In Progress` and add a GitHub issue comment explaining the blocker, unless the work never actually started.
- If the work never actually started, leave the item in `Todo`.
- If a PR is opened but not merged because of a blocker, leave the item in `In Progress`.
- In the final report for each issue, include the final project status.

Project status update mechanics:
- Use `gh project item-list 13 --owner owainlewis --format json` to find the item ID for the issue.
- Use `gh project field-list 13 --owner owainlewis --format json` to find the `Status` field ID and option IDs.
- Use `gh project item-edit --project-id <project-id> --id <item-id> --field-id <status-field-id> --single-select-option-id <option-id>` to update the status.
- Do not guess IDs if the command output is available.

Workflow for each issue:

1. Pull the ticket
- Read the GitHub issue.
- Read docs/prd.md and docs/roadmap.md.
- Read the matching project item on GitHub Project #13.
- Inspect the current repo state.
- Decide whether the issue is unblocked.
- If blocked by an earlier issue, stop and explain.
- If unblocked and you are about to work on it, move the project item to `In Progress`.

2. Plan the work
- For simple issues, write a short plan in the thread.
- For non-trivial implementation issues, create `docs/issues/<issue-number>-plan.md`.
- The plan should include:
  - Goal
  - Current repo context
  - Proposed implementation
  - Acceptance criteria
  - Verification steps
  - Out of scope

3. Create worker thread, branch, and worktree
- Create a separate Codex app worker thread for the issue using `codex_app.create_thread`.
- Use `codex_app.list_projects` first to resolve the project ID.
- Use a project target with a worktree environment.
- Use one branch per issue.
- Branch naming format: `issue-<number>-<short-slug>`.
- Use a separate worktree when helpful.
- Do not overwrite or discard unrelated local changes.

4. Implement
- The worker thread makes the smallest complete change that satisfies the issue.
- Follow existing project patterns.
- Keep docs/prd.md and docs/roadmap.md aligned if the implementation changes a decision.
- Add or update tests at the level that would catch the behavior.

5. Verify
- Run the issue’s listed verification commands.
- At minimum, run `cargo test` once the Rust project exists.
- For terminal behavior, do the manual smoke checks listed in the issue where practical.
- Clearly record anything that could not be verified.

6. Adversarial review
- Inside the worker thread, use a fresh internal subagent to review the branch before opening the PR.
- This review subagent is separate from the Codex app worker thread.
- Ask the subagent to look for:
  - Incorrect behavior vs the issue acceptance criteria
  - Missing tests
  - Terminal cleanup bugs
  - Dirty state or save bugs
  - Cursor movement edge cases
  - Over-broad scope
  - Maintainability problems
- Judge the findings.
- Fix valid in-scope findings.
- Do not blindly apply review comments that conflict with the PRD or issue scope.

7. Open PR
- Commit only intended changes.
- Push the branch.
- Open a PR against main.
- PR body must include:
  - Linked issue
  - Summary
  - Acceptance criteria checklist
  - Tests run
  - Manual verification
  - Adversarial review summary
  - Known limitations, if any

8. Merge
- You are responsible for merging.
- Before merging, ensure tests pass locally and GitHub checks are passing if checks exist.
- If checks fail, inspect and fix them.
- Use squash merge unless the repo already indicates a different preference.
- After the PR is merged, move the project item to `Done`.
- After merge, update local main before starting the next issue.

Execution order:
- Start with #1.
- Then process #2, #3, #4, #5, #6, #7, and #8 in order.
- Only parallelize if two issues are genuinely independent and can be reviewed and merged separately without conflicts.
- Prefer correctness and clean merges over parallel speed.

Stop conditions:
- Stop if an issue’s acceptance criteria are unclear.
- Stop if GitHub auth, push, PR creation, or merge permissions fail.
- Stop if a design decision materially changes docs/prd.md or docs/roadmap.md and needs human approval.
- Stop if tests reveal a deeper architectural problem outside the current issue.

Reporting after each issue:
- Issue number and title
- Project status
- Branch
- Worktree path, if used
- PR URL
- Merge commit or squash result
- Tests run
- Review findings fixed
- Anything deferred

Begin with issue #1.

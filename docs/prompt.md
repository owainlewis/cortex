Create a Codex goal for this thread:

Work through GitHub issues #1-#8 in owainlewis/cortex and turn them into merged PRs, one issue at a time, using a coordinator workflow.

Repository:
- Local path: /Users/owainlewis/Code/github/owainlewis/cortex
- GitHub repo: owainlewis/cortex
- Remote: git@github.com:owainlewis/cortex.git
- Base branch: main

Project tracking:
- GitHub Project: Cortex
- Project URL: https://github.com/users/owainlewis/projects/13
- Project number: 13
- Project owner: owainlewis
- Status lanes: Todo, In Progress, Reviewing, Done

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
- Leave the issue in `In Progress` while planning, coding, fixing, running local checks, preparing the PR, or actively addressing review feedback and failed checks.
- Move the issue to `Reviewing` after its pull request opens and while it awaits review, required checks, a decision, or merge.
- If review feedback or failed checks require active changes, move the issue back to `In Progress`, then return it to `Reviewing` after pushing the fixes.
- Move the issue to `Done` only after any required pull request or direct commit has landed on `main`, pre-finalization acceptance criteria are verified, and the issue is closed.
- Archive the Project item immediately after it reaches `Done`.
- A completed issue must remain in issue and Project archive history, not in the active Project table.
- If an issue becomes blocked, leave it in `In Progress` and add a GitHub issue comment explaining the blocker, unless the work never actually started.
- If the work never actually started, leave the item in `Todo`.
- If an open PR is blocked on review, checks, a decision, or merge, leave the item in `Reviewing`.
- In the final report for each issue, include the final project status.

Project status update mechanics:
- Use `gh project view 13 --owner owainlewis --format json --jq '.id'` to find the Project node ID.
- Use `gh project item-list 13 --owner owainlewis --limit 1000 --format json --jq '.items[] | select(.content.type == "Issue" and .content.repository == "owainlewis/cortex" and .content.number == <issue-number>) | .id'` to find the exact item ID for the issue.
- Use `gh project field-list 13 --owner owainlewis --limit 100 --format json --jq '.fields[] | select(.name == "Status")'` to find the `Status` field ID and option IDs.
- Confirm each Project, item, and field lookup returns exactly one result before changing state.
- Use `gh project item-edit --project-id <project-id> --id <item-id> --field-id <status-field-id> --single-select-option-id <option-id>` to update the status.
- Do not guess IDs if the command output is available.

Completion and archival mechanics:
- Use this finalization path for every successful completion, including a merged pull request, a direct commit, and a documentation-only or governance closure.
- Verify the completed result on `main` against every acceptance criterion that does not depend on issue closure or Project archival.
- Post an issue comment with the pull request or commit link when applicable, checks run, concrete acceptance evidence, and any finalization checks that must follow closure.
- Ensure the issue is closed.
- If a linked pull request already closed the issue, do not reopen it.
- If the issue is still open, close it only after posting the evidence comment.
- Find the current Project item, `Status` field, and `Done` option IDs from the commands above.
- Move the item to `Done` if it is not already there, without changing its other fields.
- Archive that exact item with `gh project item-archive 13 --owner owainlewis --id <item-id>`.
- Do not delete the item, reopen the issue, or change labels, milestones, or unrelated Project fields.
- Query Project items with `archivedStates: [ARCHIVED]` and confirm the exact item is still closed, has status `Done`, and has `isArchived: true`.
- Compare all paginated repository open issue URLs with all paginated active Project issue URLs in both directions:

```sh
diff -u \
  <(gh api graphql --paginate -f query='
    query($endCursor: String) {
      repository(owner: "owainlewis", name: "cortex") {
        issues(first: 100, after: $endCursor, states: OPEN) {
          nodes { url }
          pageInfo { hasNextPage endCursor }
        }
      }
    }' --jq '.data.repository.issues.nodes[].url' | sort) \
  <(gh api graphql --paginate -f query='
    query($endCursor: String) {
      user(login: "owainlewis") {
        projectV2(number: 13) {
          items(
            first: 100
            after: $endCursor
            archivedStates: [NOT_ARCHIVED]
          ) {
            nodes { content { ... on Issue { url } } }
            pageInfo { hasNextPage endCursor }
          }
        }
      }
    }' --jq '.data.user.projectV2.items.nodes[].content.url // empty' | sort)
```

- A successful comparison has no diff and equal counts.
- If the sets differ, report the URLs in each difference and stop before starting another issue.
- Never archive an open or future issue to force equality.
- Post the archived-state and equality results on the closed issue or in the coordinator’s final report.

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
- Move the project item to `Reviewing`.
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
- If review feedback or checks require fixes, move the project item to `In Progress`, make and verify the changes, then return it to `Reviewing` after pushing.
- Use squash merge unless the repo already indicates a different preference.
- After the PR is merged, run the complete finalization path under Completion and archival mechanics.
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

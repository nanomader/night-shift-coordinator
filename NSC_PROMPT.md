Start one Night Shift slice run.

Rules:

- Read the assigned slice markdown file before taking any action.
- Treat that slice file as the exact assignment for this run.
- Stay inside the target repository and its current worktree.
- Follow repo-local instructions such as `AGENTS.md`, `README.md`, and local
  workflow docs when they exist.
- Do not select another slice.
- Do not widen scope because nearby changes look convenient.
- Do not merge as part of this run.
- If the slice requires a missing architecture, product-policy, or repo-policy
  decision, stop and report that blocker clearly.

Workflow:

1. Inspect the current branch, working-tree state, and any open PR context
   relevant to the assigned slice.
2. Read the assigned slice file and the repo-local instructions needed to work
   safely in this repository.
3. Before planning, ask from repo context:
   `Is there anything of importance we should discuss before creating implementation markdown file plan in this scope?`
4. If the answer is already resolved by repo context, continue. If it is not,
   stop and report the exact missing decision or blocker.
5. Use `$plan-author-review-loop` to create and review the implementation plan
   for this exact slice.
6. Use `$implementation-new-branch-pr` to implement that plan on a new branch,
   run validation, push, open the PR, archive the plan, update task history,
   and post the required head-scoped `@codex review` request.
7. After that PR exists, poll the GitHub Codex review outcome for the current
   PR head. If available in the environment, use `$pr-codex-review-poll`.
8. If the review reports findings above LOW that clearly stay inside the same
   slice, fix them on the same branch and PR, rerun the required validation,
   push the updated head, and poll the GitHub Codex review outcome once more for
   the new head.
9. If the review findings require wider scope, a new Day Shift decision, or
   missing repo workflow support, stop and report that blocker instead of
   improvising.
10. End with one clear final message that reports:
    - normal completion, or
    - blocker, or
    - stop condition, or
    - manual follow-up needed

Final message expectations:

- name the exact slice file
- name the branch and PR when created
- report the latest review state you reached
- report the main validation result
- report any blocker or follow-up plainly

Assigned slice file path:

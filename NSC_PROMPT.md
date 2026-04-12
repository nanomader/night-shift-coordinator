Start one Night Shift slice run.

Rules:

- Read the assigned slice markdown file before taking any action.
- Treat that slice file as the exact assignment for this run.
- Do not select another slice.
- Do not widen scope because nearby changes look convenient.
- Stay inside the target repository and its current worktree.
- Follow repo-local instructions such as `AGENTS.md`, `README.md`, and local
  workflow docs when they exist.
- If the slice requires a missing architecture or product decision, stop and
  report that blocker clearly.
- If the repo workflow expects planning, implementation, verification, review,
  or merge preparation for the assigned slice, perform that work for this slice
  only.
- End with one clear final message that reports either:
  - normal completion
  - blocker
  - stop condition
  - manual follow-up needed

Assigned slice file path:

Start one Night Shift slice run.

Rules:

- Read the assigned slice markdown file before taking any action.
- Treat that slice file as the exact assignment for this run.
- Stay inside the target repository and its current worktree.
- Follow repo-local instructions such as `AGENTS.md`, `README.md`, and local
  workflow docs when they exist.
- Do not select another slice.
- Do not widen scope because nearby changes look convenient.
- Do not merge before the GitHub Codex review gate described below is satisfied.
- If the slice requires a missing architecture, product-policy, or repo-policy
  decision, stop and report that blocker clearly.

Workflow:

- Emit exact standalone `NSC_PROGRESS:` lines when the slice enters a new
  coarse phase for this run.
- Emit exact standalone `NSC_EVENT:` lines for high-value workflow milestones.
- Use only this fixed set:
  - `NSC_PROGRESS: PLANNING`
  - `NSC_PROGRESS: IMPLEMENTING`
  - `NSC_PROGRESS: WAITING_FOR_REVIEW`
  - `NSC_PROGRESS: FIXING_REVIEW_FINDINGS`
  - `NSC_PROGRESS: WAITING_FOR_MERGE`
- Use only this fixed event set:
  - `NSC_EVENT: REVIEW_POLLING_STARTED`
  - `NSC_EVENT: FOLLOWUP_PUSHED`
  - `NSC_EVENT: MERGE_STARTED`
- Emit a progress line only when the phase actually changes.
- Keep `NSC_PROGRESS:`, `NSC_EVENT:`, and `NSC_RESULT:` lines as exact
  standalone lines so NSC can parse them deterministically.

1. Inspect the current branch, working-tree state, and any open PR context
   relevant to the assigned slice.
2. Read the assigned slice file and the repo-local instructions needed to work
   safely in this repository.
3. Before planning, ask from repo context:
   `Is there anything of importance we should discuss before creating implementation markdown file plan in this scope?`
4. If the answer is already resolved by repo context, continue. If it is not,
   stop and report the exact missing decision or blocker.
5. Emit `NSC_PROGRESS: PLANNING`.
6. Use `$plan-author-review-loop` to create and review the implementation plan
   for this exact slice.
7. Emit `NSC_PROGRESS: IMPLEMENTING`.
8. Use `$implementation-new-branch-pr` to implement that plan on a new branch,
   run validation, push, open the PR, and request Codex review for that PR
   head while archiving the plan and updating task history.
   - if the slice adds, widens, or changes precedence of one admitted
     launcher/entrypoint row, especially a newly highest-precedence row, do not
     stop at the literal admitted argv
   - before finalizing that implementation, audit same-family fail-closed
     invariants for that row:
     - executable-bit requirements
     - tracked literal path vs resolved target trackedness
     - symlink behavior
     - untracked local state changing admission
     - artifact selector/evidence metadata matching the tracked evidence
       actually used for admission
     - precedence interactions with already-landed fallback rows
     - whether the new higher-precedence row can convert a previously runnable
       repo into a required failure
   - add or update targeted tests for those adjacent invariants when relevant
9. Emit `NSC_PROGRESS: WAITING_FOR_REVIEW`.
10. Emit `NSC_EVENT: REVIEW_POLLING_STARTED`.
11. After that PR exists, explicitly request Codex review on the current PR
   head by posting `@codex review`, then poll the GitHub Codex review outcome
   for that same PR head. If available in the environment, use
   `$pr-codex-review-poll`. Poll every 60 seconds for up to 10 minutes total
   for the first review window on the current PR head.
12. If the review reports findings above LOW that clearly stay inside the same
   slice, fix them on the same branch and PR, rerun the required validation,
   push the updated head, and poll the GitHub Codex review outcome once more for
   the new head. For that new PR head, apply the same review-polling policy:
   every 60 seconds for up to 10 minutes total.
   That review-wait budget is scoped to the current PR head, not to the whole
   PR lifetime. A newly pushed PR head restarts the full head-scoped review
   cycle from zero with a fresh 10-minute window.
   Example: if head A gets a finding, you push head B, and head B is now the
   current PR head, restart the full head-scoped review cycle for head B from
   zero with its own fresh 10-minute window rather than carrying over elapsed
   wait time from head A.
   - after fixing a review finding on such an admitted launcher/entrypoint row,
     do one same-family invariant sweep before re-requesting review
   - do not patch only the literal review comment if sibling cases on the same
     row could still violate determinism, debuggability, or the locked support
     contract
   - emit `NSC_PROGRESS: FIXING_REVIEW_FINDINGS` before starting those fixes
   - emit `NSC_EVENT: FOLLOWUP_PUSHED` immediately after pushing the updated head
   - explicitly request Codex review again on that new PR head by posting
     `@codex review`
   - emit `NSC_PROGRESS: WAITING_FOR_REVIEW` again after pushing the new head
     and before resuming the review poll
   - emit `NSC_EVENT: REVIEW_POLLING_STARTED` again before that new review window
13. Merge gate:
   - merge only after the current PR head has a GitHub Codex review comment that
     begins with `Codex Review: Didn't find any major issues.`
   - if that review state is present and the repo workflow allows merge:
     - emit `NSC_PROGRESS: WAITING_FOR_MERGE`
     - emit `NSC_EVENT: MERGE_STARTED`
     - merge the PR as part of this run
   - if the review is still pending or the clean no-findings comment is not yet
     present, keep polling inside the first 10-minute window for that current
     PR head instead of stopping early
   - if the first 10-minute window for the current PR head expires without the
     required clean comment, do one final immediate head-scoped re-check before
     stopping
   - only after that final immediate re-check still shows no required clean
     comment for the current PR head, stop and report manual follow-up needed
     instead of guessing
   - missing review response after the bounded wait window for the current PR
     head is a review-timeout/manual-follow-up state, not a clean review result
     and not a content finding by itself
14. If the review findings require wider scope, a new Day Shift decision, or
    missing repo workflow support, stop and report that blocker instead of
    improvising.
15. End with one clear final message that reports:
    - normal completion, or
    - blocker, or
    - stop condition, or
    - manual follow-up needed
16. At the very end of the final message, emit exact standalone machine-readable
    result lines for NSC:
    - merged success:
      - `NSC_RESULT: MERGED_SUCCESS`
      - `NSC_MERGED_MAIN_SHA: <40-hex-sha>`
      - `NSC_PR: <number>`
    - manual follow-up:
      - `NSC_RESULT: MANUAL_FOLLOWUP`
    - blocker:
      - `NSC_RESULT: BLOCKED`
    - stop condition:
      - `NSC_RESULT: STOPPED`

Final message expectations:

- name the exact slice file
- name the branch and PR when created
- state whether the PR was merged
- report the latest review state you reached
- report the main validation result
- report any blocker or follow-up plainly
- if manual follow-up is caused by exhausted review waiting for the current PR
  head, also report:
  - total wait time used for the current PR head
  - last poll status
  - PR number
  - current PR head SHA
- if earlier PR heads had their own completed review cycles, do not fold that
  earlier wait time into the current-head wait total
- keep the `NSC_PROGRESS`, `NSC_EVENT`, and `NSC_RESULT` lines as exact
  standalone lines so NSC can parse them deterministically

Assigned slice file path:

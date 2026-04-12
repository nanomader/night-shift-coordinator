# Repo Guidance

- Never use absolute local filesystem paths such as `/Users/...`, `/home/...`, or `/private/...` in code, docs, prompts, plans, comments, or tests.
- When referencing repo files, use repo-relative paths such as `src/worktree.rs` or markdown links like `[src/worktree.rs](src/worktree.rs)`.
- Do not include personal machine identifiers, local usernames, or home-directory paths in committed content.
- If a GitHub identity is needed in committed text, use the public handle `nanomader`, not a personal name derived from a local path or workstation config.

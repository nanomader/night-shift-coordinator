# Contributing To Night Shift Coordinator

Thanks for contributing to Night Shift Coordinator (`NSC`).

Before changing behavior, read these files first:

- [README.md](README.md)
- [AGENTS.md](AGENTS.md)
- [NSC_PROMPT.md](NSC_PROMPT.md)

Treat `README.md` and `AGENTS.md` as the active product contract.

## Boundary

NSC is a local Rust CLI for supervising prepared Night Shift work.

Keep this boundary intact:

- NSC executes prepared work
- NSC does not prepare, repair, or reinterpret that work
- ordered execution is explicit only
- deterministic discovery and execution truth outrank convenience

Changes that add planning, dependency inference, hidden fallbacks, repair
layers, or general workflow-engine behavior are out of scope unless the product
contract changes first.

## Before You Open A PR

Make sure your change updates all affected surfaces together:

- implementation
- tests
- docs
- [NSC_PROMPT.md](NSC_PROMPT.md) if the prompt contract changed
- recovery or parsing code if the runtime contract changed

Useful local commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Review Standard

Prefer small, explicit changes that preserve:

- deterministic execution
- fail-fast validation
- explicit operator-visible contracts
- recovery clarity

If a behavior matters during unattended runs, optimize for debuggability over
cleverness.

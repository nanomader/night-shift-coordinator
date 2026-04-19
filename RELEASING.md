# Releasing NSC

This file is the release checklist for Night Shift Coordinator.

Use it when you are preparing a tagged GitHub release or handing a version to
the separate Homebrew tap.

## Before You Start

- work from `main`
- make sure tracked files are committed
- make sure `Cargo.toml` has the version you want to release

## Required Validation

Run these commands before you create a tag:

```bash
cargo fmt --check
cargo test --quiet
cargo install --path . --locked --root /tmp/nsc-release-smoke
/tmp/nsc-release-smoke/bin/nsc --help
```

If any command fails, stop and fix that first.

## Create The Tag

NSC tags use the form `vX.Y.Z`.

Example for version `0.1.0`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Publish The GitHub Release

After the tag is pushed, create the GitHub release:

```bash
gh release create v0.1.0 --title "v0.1.0" --generate-notes
```

Replace `0.1.0` with the real version each time.

## Homebrew Tap Handoff

The tap should build from the tagged source archive, not from a prebuilt NSC
binary.

Tagged source tarball URL pattern:

```text
https://github.com/nanomader/night-shift-coordinator/archive/refs/tags/vX.Y.Z.tar.gz
```

Checksum commands:

- macOS:

```bash
curl -L https://github.com/nanomader/night-shift-coordinator/archive/refs/tags/v0.1.0.tar.gz -o /tmp/nsc.tar.gz
shasum -a 256 /tmp/nsc.tar.gz
```

- Linux:

```bash
curl -L https://github.com/nanomader/night-shift-coordinator/archive/refs/tags/v0.1.0.tar.gz -o /tmp/nsc.tar.gz
sha256sum /tmp/nsc.tar.gz
```

Repo-side values the tap maintainer will need:

- package name: `night-shift-coordinator`
- binary name: `nsc`
- description: `Local Rust CLI for supervising prepared Night Shift slice execution`
- homepage: `https://github.com/nanomader/night-shift-coordinator`
- license: `Apache-2.0`
- smoke test: `nsc --help`
- help output check string: `Supervise prepared Night Shift slice execution`

Runtime requirements that Homebrew will not install for the operator:

- `git`
- `codex` with App Server support
- GitHub access for the default built-in prompt flow

For the full handoff note, see [docs/homebrew-tap.md](docs/homebrew-tap.md).

## Bad Release Rollback

If you created a bad tag before anyone depends on it, remove the GitHub release
and the tag:

```bash
gh release delete v0.1.0 --yes
git push origin :refs/tags/v0.1.0
git tag -d v0.1.0
```

If the bad version is already in use, prefer a new patch release instead of
rewriting history.

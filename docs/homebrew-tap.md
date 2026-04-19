# Homebrew Tap Handoff

This repository does not own the Homebrew formula.

The formula should live in a separate tap repository. This document records the
repo-side facts the tap maintainer should not need to rediscover.

## Current Package Values

- package name: `night-shift-coordinator`
- binary name: `nsc`
- description: `Local Rust CLI for supervising prepared Night Shift slice execution`
- homepage: `https://github.com/nanomader/night-shift-coordinator`
- license: `Apache-2.0`

## Source Archive

The tap should point at the tagged GitHub source tarball:

```text
https://github.com/nanomader/night-shift-coordinator/archive/refs/tags/vX.Y.Z.tar.gz
```

Get the SHA256 from the tagged tarball, not from a local working tree archive.

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

## Expected Formula Shape

The tap formula should build from source and keep the test simple.

- build from the tagged source tarball
- install with Cargo
- use `nsc --help` in `test do`

Example build and test shape:

```ruby
depends_on "rust" => :build

def install
  system "cargo", "install", *std_cargo_args(path: ".")
end

test do
  assert_match "Supervise prepared Night Shift slice execution", shell_output("#{bin}/nsc --help")
end
```

## Runtime Caveats

Homebrew will install the `nsc` binary. It will not prepare the full runtime
environment for the operator.

Keep these caveats visible in the tap formula or tap documentation:

- `git` is required at runtime
- `codex` with App Server support is required at runtime
- the default built-in prompt flow also requires GitHub access that can create
  pull requests, post `@codex review`, inspect review state, and merge when the
  target repository workflow allows it

If the operator does not have that GitHub workflow capability, they should use
`--prompt-template` and provide a prompt that matches their repository process.

## Install Goal

Target install after the separate tap exists:

```bash
brew install nanomader/tap/night-shift-coordinator
```

Plain `brew install night-shift-coordinator` is out of scope unless NSC later
enters `homebrew/core`.

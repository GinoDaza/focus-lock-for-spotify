# ADR 0009: CI and releases

- Status: accepted
- Date: 2026-10-16

## Context

Contributors need the same checks the maintainer runs by hand, and users need to be able to trust that an installer comes from the source in the repository. A hand-built installer uploaded by the maintainer can't be checked by anyone else.

## Decision

- `ci.yml` runs on pushes to `main` and on pull requests. It checks the extension and the desktop frontend (typecheck, tests, build), and Rust formatting, `clippy -D warnings` and tests on Linux (without the desktop app) and on Windows (the whole workspace).
- `release.yml` runs only when a tag `v*` is pushed. It checks that the tag matches the crate version, builds the extension, the service, the CLI and the NSIS installer on a Windows runner with `tauri-action`, and attaches the installer to a **draft** release. The maintainer reviews the draft, adds the notes and publishes it.
- Versions follow semver, starting at `v0.x`.
- The Windows build targets `x86_64-pc-windows-msvc` explicitly, so `tauri.conf.json` keeps one set of resource paths for local and CI builds.
- Code signing with SignPath is added to the release workflow once the project is approved. Until then, the installer is unsigned.

## Consequences

- Anyone can read the build log of a release and compare it with the tagged source.
- A release is two steps for the maintainer: create the tag, then review and publish the draft.
- The workflows can't be tested locally, so the first runs may need adjustments.

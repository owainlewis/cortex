# Release Workflow

This document explains how to publish stable releases and test nightly builds.

## Stable Releases

Stable releases are built from version tags.
Use tags in the form `vX.Y.Z`, for example `v0.1.0`.

Before tagging, make sure `main` is green in GitHub Actions.
Run the local checks you need for the change you are releasing.
Update the package version in `Cargo.toml` before tagging when the release version changes.

Create and push the tag:

```sh
git checkout main
git pull --ff-only origin main
git tag v0.1.0
git push origin v0.1.0
```

Pushing the tag starts the `Release` workflow.
The workflow builds the macOS release binary and creates a GitHub Release.
It uploads these assets:

- `cortex-vX.Y.Z-aarch64-apple-darwin.tar.gz` or `cortex-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- a matching `.sha256` checksum file

The release workflow refuses to publish if a release for the tag already exists.
It should not overwrite existing release assets.

Verify the release after the workflow completes:

```sh
gh release view v0.1.0 --repo owainlewis/cortex
```

Download the archive and checksum.
Verify the checksum before running the binary:

```sh
shasum -a 256 -c cortex-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf cortex-v0.1.0-aarch64-apple-darwin.tar.gz
./cortex --version
```

## Install And Update

Users install the latest stable release by running the installer:

```sh
curl -fsSL https://raw.githubusercontent.com/owainlewis/cortex/main/install.sh | bash
```

The installer fetches the latest GitHub Release, downloads the archive and checksum, verifies the checksum, and installs the binary.
Users update by running the installer again.

The binary also has explicit version and update-check commands:

```sh
cortex --version
cortex --check-update
```

`cortex --check-update` only reports whether a newer stable release exists.
It does not replace the installed binary.

## Nightly Builds

Nightly builds are for quick testing between stable releases.
They are unstable and may be broken.

The `Nightly` workflow can be run manually from GitHub Actions.
It also runs on a schedule.
It checks out `main`, builds the macOS release binary, and uploads a workflow artifact.

The uploaded artifact is named with `cortex-nightly`, the workflow run number, and the macOS target triple.
The archive and checksum inside that artifact also include the commit SHA.
They are retained for 14 days.
They are not GitHub Releases.
They are not used by `install.sh`.

To verify a nightly artifact, download it from the workflow run and check the checksum:

```sh
shasum -a 256 -c cortex-nightly-*.tar.gz.sha256
tar -xzf cortex-nightly-*.tar.gz
./cortex --version
```

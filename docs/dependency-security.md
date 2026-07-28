# Dependency security

Cortex uses GitHub Dependabot and a RustSec audit to monitor dependency vulnerabilities.

## Repository settings

Dependabot vulnerability alerts and Dependabot security updates must remain enabled in the GitHub repository settings.
Secret scanning and push protection remain separate controls and must not be weakened when changing dependency security settings.

## Automated updates

`.github/dependabot.yml` checks Cargo and GitHub Actions dependencies every Monday.
Cargo patch updates are grouped, while Cargo minor and major updates remain separate because most direct dependencies use `0.x` versions.
GitHub Actions minor and patch updates are grouped, while major updates remain separate.
Each ecosystem is limited to three open version-update pull requests.
Dependabot security updates are managed by GitHub and may be opened outside the version-update schedule.

## Vulnerability audit

`.github/workflows/security.yml` runs `cargo audit` against the committed `Cargo.lock` every Monday, when Cargo dependency files change on `main`, and on manual dispatch.
Scheduled runs audit `main`, while manual runs audit the selected revision.
It runs separately from normal pull-request checks because the RustSec advisory database changes independently of a pull request.
This keeps routine pull-request checks deterministic while still detecting newly published advisories.

Audit concurrency is grouped by event name and Git ref.
Scheduled, `main` push, and manual audits therefore cannot cancel each other, and manual audits of different refs remain independent.
Cancellation stays enabled within one event and ref so a newer duplicate can replace an older run.

The audit fails for every active RustSec vulnerability that affects the lockfile, regardless of CVSS score.
There is no minimum score because RustSec advisories do not all carry comparable severity metadata.
Informational warnings, including unmaintained, unsound, or yanked-package notices, are reported in the job log but do not fail the audit.
Any ignored advisory must be documented here with an owner, reason, and expiry date.
There are currently no ignored advisories.

Run the same audit locally with:

```sh
cargo install cargo-audit --locked --version 0.22.2
cargo audit
```

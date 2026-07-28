# Contributing

Cortex is a small macOS-only terminal editor.
Keep changes focused and consistent with `AGENTS.md`, `docs/prd.md`, and `docs/roadmap.md`.

## Before You Start

- Search existing issues and discussions before opening a new one.
- Use GitHub Discussions for questions and early ideas.
- Use the bug or feature issue form for scoped work.
- Report security vulnerabilities privately as described in `SECURITY.md`.
- Discuss non-trivial changes in an issue before writing code.

## Local Setup

Development requires macOS and the stable Rust toolchain.
Install Rust with [rustup](https://rustup.rs/).
Install the Xcode command-line tools with `xcode-select --install` if the compiler or linker is missing.

Fork the repository, then clone your fork:

```sh
git clone https://github.com/YOUR-USER/cortex.git
cd cortex
rustup default stable
cargo build
```

Run Cortex with a file or directory:

```sh
cargo run -- path/to/file.txt
cargo run -- .
```

## Scope

- Keep Cortex macOS-only.
- Prefer the smallest complete change that solves the linked issue.
- Do not add broad configuration, plugins, scripting, LSP, or cross-platform work unless the roadmap explicitly calls for it.
- Keep documentation aligned with shipped behavior and product direction.
- Add focused tests when behavior changes, especially for pure editor logic.

## Checks

Run the same checks required by CI:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

For terminal-facing changes, also run a manual smoke test:

```sh
cargo run -- /tmp/cortex-smoke.txt
```

Open the file, edit it, save it, and quit.
Confirm the terminal is restored and the shell remains usable after Cortex exits.

## Pull Requests

- Use a focused branch and a Conventional Commit-style title such as `fix: restore the cursor after reload`.
- Complete the pull request template.
- Link the issue the pull request closes.
- Explain the change, test evidence, and any remaining risk.
- Update tests and documentation in the same pull request when behavior changes.
- Keep unrelated cleanup out of the diff.
- Pull requests are squash merged.
- GitHub automatically deletes merged head branches hosted in this repository.

Pull requests must pass the required `Rust checks` workflow before merge.

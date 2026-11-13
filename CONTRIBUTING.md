# Contributing

Thanks for your interest in contributing to SolideoWorship! We appreciate your time and help.

Before starting on a new feature or non-trivial change, please open or check [GitHub Issues](https://github.com/MikeNoppo/solideoworship-app/issues) to discuss it first. Also read [AGENTS.md](AGENTS.md) for the architecture, performance rules, and platform pitfalls.

- [Commit Convention](#commit-convention)
- [Setup](#setup)
- [Development](#development)
- [Translations](#translations)
- [Pull Request](#pull-request)
- [Important Considerations](#important-considerations-for-pull-requests)

## Commit Convention

Commit messages must follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Allowed types:

- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Formatting or whitespace changes that do not affect code behavior
- **refactor**: Code change that neither fixes a bug nor adds a feature
- **perf**: Performance improvement
- **test**: Adding or updating tests
- **build**: Changes affecting the build system or dependencies
- **ci**: Changes to CI workflows or scripts
- **chore**: Maintenance tasks that do not modify application or test code
- **revert**: Reverts a previous commit

Example:

```
feat(app): add keyboard shortcut for next slide
```

## Setup

Fork the repository and create your branch from `main`.

Prerequisites:

- [Rust](https://rustup.rs) (`rustup` automatically installs the pinned toolchain from `rust-toolchain.toml`)
- A C compiler for bundled SQLite:
  - **Windows:** Visual Studio Build Tools
  - **macOS:** Xcode Command Line Tools
  - **Linux:** `libxcb1-dev` and standard C build tools

```sh
git clone https://github.com/MikeNoppo/solideoworship-app.git
cd solideoworship-app
cargo run -p sdw-app
```

## Development

Useful workspace commands:

```sh
cargo check                  # fast type check
cargo test --workspace       # run all tests, including SQLite in-memory tests
cargo clippy --all-targets   # run linter
cargo fmt                    # format code
```

CI runs on every push to `main` and every pull request across Linux, Windows, and macOS:

- `cargo clippy --workspace --all-targets --locked --profile ci -- -D warnings`
- `cargo test --workspace --locked --profile ci`
- `cargo check --workspace --all-targets --locked --profile ci` against the MSRV (`rust-version` in `Cargo.toml`)
- `cargo fmt --all --check` and [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) on Linux

Additional conventions:

- Any new dependency must come from crates.io with a GPL-3.0-compatible license listed in `deny.toml`.
- Code, identifiers, comments, and documentation are written in English.
- UI strings live in locale JSON files, never as string literals in code.
- Performance and memory efficiency come first: never load the entire library into memory, and borrow text in `view` instead of cloning.
- Note: the `block v0.1.6` future-incompatibility warning comes from a transitive macOS dependency of `display-info`, not this repository.

## Translations

UI copy lives in [`crates/app/locales`](crates/app/locales), one JSON file per language, using `en.json` as the source of truth.

1. Copy `en.json` to `<code>.json` (for example `de.json`) and translate the values while keeping keys and `{placeholders}` intact.
2. Run `cargo build`; the new language is registered automatically.

Counts use `_one`/`_other` key pairs. The build fails on unknown keys, mismatched placeholders, or leading/trailing whitespace; missing values fall back to English with a build warning.

## Pull Request

- Target the `main` branch and create a dedicated branch for each feature or bug fix.
- Add or update unit tests for behavior changes.
- Update relevant documentation if behavior or setup changes.
- Provide a clear description of what changed and why. Include screenshots or recordings for UI changes.
- Reference any related issue in the PR description (e.g., `Fixes #123`).

### Important Considerations for Pull Requests

- **Testing is Mandatory:** Every Pull Request **must be checked and tested** locally (`cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace`) before submission.
- **Focus and Scope:** Keep each PR focused on a single, well-defined fix or feature.
- **Avoid Unfocused Changes:** Avoid PRs containing only unrelated whitespace, formatting, or drive-by cleanup changes.
- **Large Features:** Large or architectural features should be discussed and agreed upon in a GitHub Issue before implementation.

Thank you for contributing!

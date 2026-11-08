# Contributing

Thanks for helping. [AGENTS.md](AGENTS.md) describes the architecture, the performance rules, and the platform pitfalls already hit; read it before larger changes.

## Development

```sh
cargo check                  # fast type check
cargo test --workspace       # every test, including SQLite in memory
cargo clippy --all-targets   # lint
cargo fmt                    # format
```

CI runs on every push to `main` and every pull request: clippy with no warnings, the tests, and `cargo check` with the minimum Rust version (`rust-version` in `Cargo.toml`) on Linux, Windows, and macOS, plus `cargo fmt --check` and [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) on Linux. A new dependency must come from crates.io with a GPL-3.0-compatible license listed in `deny.toml`. The `block v0.1.6` future-incompatibility warning comes from a macOS dependency of `display-info`, not from this code.

## Conventions

- Commits follow [Conventional Commits](https://www.conventionalcommits.org), one logical change per commit.
- Code, identifiers, comments, and docs are in English.
- UI text goes in the locale files, never as literals in code.
- Performance and memory use come first: don't load the whole library into memory, and don't clone text inside `view`.

## Translations

UI text lives in [`crates/app/locales`](crates/app/locales), one JSON file per language, with `en.json` as the source of truth.

1. Copy `en.json` to `<code>.json` (e.g. `de.json`) and translate the values, keeping keys and `{placeholders}` unchanged.
2. Run `cargo build`; the new language is picked up automatically.

Counts use `_one`/`_other` pairs. The build fails on an unknown key, mismatched placeholders, or leading/trailing whitespace; a missing value falls back to English with a warning.

## Releasing

1. Bump `version` in the workspace `Cargo.toml`, run `cargo check` so `Cargo.lock` follows, and commit.
2. Push a matching tag: `git tag v0.2.0 && git push origin v0.2.0`.
3. The Release workflow builds the Windows installer and the Linux archive into a **draft** GitHub release with `SHA256SUMS.txt`. Check it, then publish.

Starting the workflow by hand from the Actions tab builds both packages as run artifacts without a release. To package locally:

- Linux: `packaging/linux/package.sh <version>` writes the archive to `target/dist/`.
- Windows, with [Inno Setup](https://jrsoftware.org/isinfo.php) 7: `cargo build --release -p sdw-app`, then `ISCC /DAppVersion=<version> packaging\windows\solideoworship.iss` writes the installer to `target\installer\`. Leave `RUSTFLAGS` unset; it replaces the static C runtime setting in `.cargo/config.toml`.

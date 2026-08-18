# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`solideoworship-app` — a native desktop GUI application built with [iced](https://iced.rs) 0.14 on Rust edition 2024. The repo is currently at scaffold stage: [src/main.rs](src/main.rs) holds the stock iced counter example and is the only source file. Treat the real application structure as still to be designed.

## Commands

```sh
cargo run              # build + launch the GUI
cargo build --release  # optimized build
cargo check            # fast type-check, no codegen (use while iterating)
cargo clippy           # lints
cargo fmt              # format
cargo test             # no tests exist yet
cargo test <name>      # run a single test by (substring of) its path
```

## Architecture

iced follows the Elm architecture. Every screen or component is the same four pieces:

1. **State** — a struct holding everything the UI renders from.
2. **Message** — an enum of every event the UI can emit (`#[derive(Debug, Clone)]` is required; iced clones messages internally).
3. **`update(&mut self, message: Message)`** — the only place state mutates. Return `Task<Message>` instead of `()` when the update needs to trigger async work or commands.
4. **`view(&self) -> impl Into<Element<Message>>`** — pure function from state to widgets; it must not mutate state.

`main()` wires them together with `iced::run(update, view)`.

### iced 0.14 API notes

Most iced tutorials and LLM training data target 0.12/0.13 and will not compile here. In 0.14:

- There is no `Sandbox` or `Application` trait to implement — an app is just an update fn and a view fn.
- `iced::run(update, view)` takes exactly two arguments and requires `State: Default`. It is shorthand for `iced::application(State::default, update, view).run()`.
- Title, theme, subscriptions, and window size are set by builder chaining on `iced::application(...)` (`.title(..)`, `.theme(..)`, `.subscription(..)`, `.window_size(..)`) — not by trait methods and not as extra args to `run`. Switch from `run` to `application` as soon as any of those are needed.
- `Command` no longer exists; async work returns `Task`.
- Borrowed widget trees need an explicit lifetime: `Column<'_, Message>`.

When unsure about an API, check the version resolved in `Cargo.lock` and the corresponding docs.rs page rather than assuming.

## Conventions

- Code comments in this repo are written in Indonesian. Match that when adding comments (and see the global comment rules — comments explain *why*, not *what*).

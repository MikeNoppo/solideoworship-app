# AGENTS.md

This file provides guidance to Codex (claude.ai/code) when working with code in this repository.

## Project

`solideoworship-app` — a worship presenter application (EasyWorship-style) built with [iced](https://iced.rs) 0.14 on Rust edition 2024. The operator drives slides from a control window on the laptop; the congregation sees them on a projector via a second, fullscreen output window.

Locked scope decisions: MVP covers **songs/lyrics and Bible verses only** (images and video deferred); **macOS and Windows are equal targets**, tested each milestone; no import of existing EasyWorship/OpenLP libraries yet; **runtime performance and memory efficiency are the stated priority**.

## Commands

```sh
cargo run -p sdw-app     # build + launch the app
cargo check              # fast type-check while iterating
cargo test -p sdw-core   # pure domain logic — no GUI needed
cargo clippy --all-targets
cargo fmt
```

`cargo clippy` reports a `block v0.1.6` future-incompatibility warning. It comes from a transitive macOS dependency of `display-info`, not from this code.

## Architecture

Cargo workspace. The split exists to keep iced out of the domain layer's rebuild path — iced is slow to compile, so changing storage or parsing logic should not trigger it.

- `crates/core` (`sdw-core`) — domain types and pure logic. **No iced, no rusqlite.** Unit-testable without a GUI.
- `crates/app` (`sdw-app`) — the iced daemon binary.
- `crates/library` (`sdw-library`) — planned: SQLite storage via `rusqlite` (`bundled` feature, FTS5). Not created yet.

### Two windows, one state

The app is an `iced::daemon`, not an `iced::application`, because it drives two windows. `view(&self, window::Id)` dispatches on `WindowRole` looked up in `App::roles`; both windows render from the same state, so the projector can never drift out of sync with the operator's preview.

### Planned unifying abstraction

Songs and scripture both compile into a `SlideDeck` of `Slide`s. The live engine and the renderer are meant to know only `Slide` — never where it came from. Adding images, video, or slide decks later should mean adding a compiler, not touching the live engine. Keep this boundary intact.

### Performance rules (deliberate, not incidental)

1. Never load the whole library into memory. Song lists hold `(id, title)`; full lyrics load on selection. Bible verses are queried per displayed range.
2. Prefer `Box<str>`/`Arc<str>` over `String` for immutable text; share decks with `Arc`, do not clone contents.
3. `view` must borrow, not clone: `text(&slide.body)` yields `Fragment::Borrowed`; `text(slide.body.clone())` allocates on every redraw.
4. `scrollable` does not virtualize — every child is laid out. Bound search results, and wrap long lists in `lazy`.
5. Reactive rendering is on by default in 0.14. Do not add per-frame subscriptions unless animating, and never enable the `unconditional-rendering` feature.

## Platform pitfalls already hit (verified by testing, not assumed)

**Projector fullscreen must be set at window creation.** `window::open` with windowed settings followed by `window::set_mode(id, Mode::Fullscreen)` is flaky on macOS: winit's `set_fullscreen` silently returns for `Fullscreen::Borderless(None)` when `current_monitor_inner()` is still `None`, which happens when the window is not yet attached to a screen. Measured 1 failure in 3 runs. Setting `fullscreen: true` together with `position` in `window::Settings` succeeded 3 of 3. See [windows.rs](crates/app/src/windows.rs). Do not set `level` or `resizable` on the output window — macOS resets the level and forces a `Titled|Resizable` style mask when entering fullscreen.

**`display-info` coordinate units differ by platform.** macOS reports `CGDisplayBounds` (logical points) and Linux divides by scale factor itself, but Windows reports `DEVMODE` physical pixels. iced always treats `Position::Specific` as logical points, so only Windows needs dividing. See [display.rs](crates/app/src/display.rs).

**iced has no monitor enumeration** — only `window::monitor_size` for the window's current monitor. Hence the `display-info` dependency.

**iced text has no outline or shadow.** `core::text::Text` has no such field. Lyric readability over busy backgrounds needs a scrim/panel or offset text layers.

## iced 0.14 API notes

Most iced material online targets 0.12/0.13 and will not compile here.

- No `Sandbox`/`Application` trait to implement — an app is an update fn and a view fn.
- `iced::run(update, view)` takes two args and requires `State: Default`; it is shorthand for `iced::application(State::default, update, view).run()`. Title, theme, subscription, and scale factor are builder methods on `application(..)`/`daemon(..)`.
- `Command` no longer exists; async work returns `Task`.
- Borrowed widget trees need an explicit lifetime: `Column<'_, Message>`.

Check the version resolved in `Cargo.lock` and read the crate source in `~/.cargo/registry` rather than assuming an API exists.

## Conventions

Code comments are written in Indonesian, as is UI copy. Follow the global comment rules — comments explain *why*, not *what*.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`solideoworship-app` — a worship presenter application (EasyWorship-style) built with [iced](https://iced.rs) 0.14 on Rust edition 2024. The operator drives slides from a control window on the laptop; the congregation sees them on a projector via a second, fullscreen output window.

Locked scope decisions: MVP covers **songs/lyrics and Bible verses only** (images and video deferred); **macOS and Windows are equal targets**; no import of existing EasyWorship/OpenLP libraries yet; **runtime performance and memory efficiency are the stated priority**.

## Commands

```sh
cargo run -p sdw-app     # build + launch the app
cargo check              # fast type-check while iterating
cargo test -p sdw-core   # pure domain logic — no GUI needed
cargo test --workspace   # everything, including SQLite against an in-memory database
cargo clippy --all-targets
cargo fmt
```

`cargo clippy` reports a `block v0.1.6` future-incompatibility warning. It comes from a transitive macOS dependency of `display-info`, not from this code.

CI (`.github/workflows/ci.yml`) runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace` on Linux, Windows, and macOS, `cargo check` on the `rust-version` toolchain, and `cargo deny check`. The local toolchain is usually newer than `rust-version`, so a std API stabilized after 1.92 compiles locally and fails CI; check with `cargo +1.92 check --workspace --all-targets`. A new dependency must carry a license from the allow list in `deny.toml`, all of which are GPL-3.0-compatible. Linux builds need `libxcb1-dev` because `display-info` links libxcb.

## Architecture

Cargo workspace. The split exists to keep iced out of the domain layer's rebuild path — iced is slow to compile, so changing storage or parsing logic should not trigger it.

- `crates/core` (`sdw-core`) — domain types and pure logic. **No iced, no rusqlite.** Unit-testable without a GUI.
- `crates/app` (`sdw-app`) — the iced daemon binary.
- `crates/library` (`sdw-library`) — SQLite storage via `rusqlite` (`bundled` feature, FTS5). One `Library` struct is the whole interface; `rusqlite` types never appear in it. Tests go through `Library::open_in_memory()`, not a trait.

Bibles are imported from Zefania XML by a streaming `quick-xml` reader (`zefania.rs`) inside one transaction, so a broken file leaves nothing behind. `NOTE`/`XREF` content is dropped so footnotes never reach the projector. Verses live in a `WITHOUT ROWID` table keyed by `(bible_id, book, chapter, verse)`; a passage lookup is one primary-key range scan.

Schema changes are appended to `MIGRATIONS` in `migrate.rs` and tracked with `PRAGMA user_version`. Never edit a migration that has shipped — testers keep their databases between milestones.

### Two windows, one state

The app is an `iced::daemon`, not an `iced::application`, because it drives two windows. `view(&self, window::Id)` dispatches on `WindowRole` looked up in `App::roles`; both windows render from the same state, so the projector can never drift out of sync with the operator's preview.

### Planned unifying abstraction

Songs and scripture both compile into a `SlideDeck` of `Slide`s. A schedule entry keeps its source `ScheduleItem` next to the compiled deck, so decks can be recompiled when slide limits change. The live engine and the renderer are meant to know only `Slide` — never where it came from. Adding images, video, or slide decks later should mean adding a compiler, not touching the live engine. Keep this boundary intact.

### Performance rules (deliberate, not incidental)

1. Never load the whole library into memory. Song lists hold `(id, title)`; full lyrics load on selection. Bible verses are queried per displayed range.
2. Prefer `Box<str>`/`Arc<str>` over `String` for immutable text; share decks with `Arc`, do not clone contents.
3. `view` must borrow, not clone: `text(&slide.body)` yields `Fragment::Borrowed`; `text(slide.body.clone())` allocates on every redraw.
4. `scrollable` does not virtualize — every child is laid out. Bound result lists (`SONG_LIMIT` for song search, `PREVIEW_LIMIT` for the Bible tab). Do not reach for `lazy` to fix a long list: in 0.14 it only skips rebuilding the subtree, layout still runs every pass (`Widget::layout` in `iced_widget-0.14.2/src/lazy.rs`), and its content must be owned, which breaks rule 3. Slide lists of a deck stay unbounded because the operator must reach every slide. Measured on a release build (Xvfb software rendering, i5-12600H): one slide change costs about 56 ms CPU with Mzm 119 (176 slides) in Preview and Live versus 52 ms with a 5-slide deck, so rendering dominates and the lists add roughly 3.5 ms. Re-measure on a low-end laptop before adding custom virtualization.
5. Reactive rendering is on by default in 0.14. Do not add per-frame subscriptions unless animating, and never enable the `unconditional-rendering` feature.

## Platform pitfalls already hit (verified by testing, not assumed)

**Projector fullscreen must be set at window creation.** `window::open` with windowed settings followed by `window::set_mode(id, Mode::Fullscreen)` is flaky on macOS: winit's `set_fullscreen` silently returns for `Fullscreen::Borderless(None)` when `current_monitor_inner()` is still `None`, which happens when the window is not yet attached to a screen. Measured 1 failure in 3 runs. Setting `fullscreen: true` together with `position` in `window::Settings` succeeded 3 of 3. See [windows.rs](crates/app/src/windows.rs). Do not set `level` or `resizable` on the output window — macOS resets the level and forces a `Titled|Resizable` style mask when entering fullscreen.

**`display-info` coordinate units differ by platform.** macOS reports `CGDisplayBounds` (logical points) and Linux divides by scale factor itself, but Windows reports `DEVMODE` physical pixels. iced always treats `Position::Specific` as logical points, so only Windows needs dividing. See [display.rs](crates/app/src/display.rs).

**`Cmd+Q` on macOS cannot be intercepted.** winit's default app menu (iced never disables it) calls `-[NSApp terminate:]`; winit then closes every window directly and exits without a `CloseRequested`, so the unsaved-changes prompt never runs. That is why the schedule is mirrored to `recovery.sdw` whenever it is dirty ([recovery.rs](crates/app/src/recovery.rs)) and restored on the next start.

**iced has no monitor enumeration** — only `window::monitor_size` for the window's current monitor. Hence the `display-info` dependency.

**iced text has no outline or shadow.** `core::text::Text` has no such field. Lyric readability over busy backgrounds needs a scrim/panel or offset text layers.

## iced 0.14 API notes

Most iced material online targets 0.12/0.13 and will not compile here.

- No `Sandbox`/`Application` trait to implement — an app is an update fn and a view fn.
- `iced::run(update, view)` takes two args and requires `State: Default`; it is shorthand for `iced::application(State::default, update, view).run()`. Title, theme, subscription, and scale factor are builder methods on `application(..)`/`daemon(..)`.
- `Command` no longer exists; async work returns `Task`.
- Borrowed widget trees need an explicit lifetime: `Column<'_, Message>`.
- **Never block inside `Task::perform`.** With the `smol` feature, task futures, subscription streams, and event forwarding share smol's single executor thread, and events reach `listen_with` through a bounded channel that drops on overflow. A slow query there loses operator key presses. SQLite goes through `work::Store::run` (one dedicated thread, FIFO); other blocking work goes through `work::background`.
- Native file dialogs (`rfd`) are built inside `window::run(control_id, ..)`. That closure runs on the event-loop thread, which macOS requires for AppKit, and it parents the dialog to the control window. The returned future is then awaited with `Task::perform`.
- `text_input` without `on_submit` lets Enter through as `Status::Ignored`, and `text_editor` never captures Escape. Global shortcuts would fire while typing, so every `text_input` gets an `on_submit` (in dialogs it submits the form), and while a dialog is open the subscription swaps to dialog-only shortcuts (no Escape mapping).
- Async results that belong to a dialog carry `App::modal_ticket` from when the request was sent. Apply them to the open dialog only when the ticket still matches; the operator may have closed it and opened another meanwhile.

Check the version resolved in `Cargo.lock` and read the crate source in `~/.cargo/registry` rather than assuming an API exists.

## Conventions

Code comments are written in Indonesian, as is UI copy. Follow the global comment rules — comments explain *why*, not *what*.

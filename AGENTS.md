# AGENTS.md

This file provides guidance to coding agents (Codex, Claude Code, and others) when working with code in this repository. `CLAUDE.md` only imports this file, so edit this one.

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

CI (`.github/workflows/ci.yml`) runs on every push to `main` and every pull request. On Linux, Windows, and macOS it runs `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` with the toolchain pinned in `rust-toolchain.toml`, plus `cargo check --all-targets` with the MSRV (`rust-version` in `Cargo.toml`). All three use `--profile ci`, which skips dependency optimization and debug info. On Linux only it runs `cargo fmt --check` and `cargo deny check`; the advisories check does not block pull requests and also runs weekly against `main`.

Bump `rust-toolchain.toml` deliberately: a new stable can add clippy lints that fail CI. A `RUSTUP_TOOLCHAIN` environment variable overrides that file. The pinned toolchain is newer than the MSRV, so a std API stabilized after `rust-version` compiles locally and fails CI; check with `cargo +<rust-version> check --workspace --all-targets`. A new dependency must come from crates.io and carry a license from the allow list in `deny.toml`, all of which are GPL-3.0-compatible. Linux builds need `libxcb1-dev` because `display-info` links libxcb.

Releases are built by `.github/workflows/release.yml` on a `v*` tag (the tag must equal the workspace `version`) or by hand through `workflow_dispatch`, which only uploads artifacts. A tag produces a **draft** GitHub release; publishing stays manual. A rerun replaces the draft's files and refuses to touch a published release.

- `packaging/windows/solideoworship.iss` is an Inno Setup 7 script. Never change its `AppId` GUID: Windows uses it to recognize upgrades and the existing uninstaller. Its `[Languages]` entry names equal the app's language codes, so `ActiveLanguage` is written verbatim to `{app}\language`. `Indonesian.isl` is the unofficial translation from jrsoftware.org, vendored; it has exactly the message keys of `Default.isl` at `is-7_1_1` (both are headed "6.5.0+", the message format version), so compare the key sets when bumping Inno Setup. `CloseApplications=force` is deliberate: winit 0.30 has no handler for the `WM_QUERYENDSESSION`/`WM_ENDSESSION` that the Restart Manager sends, so a plain request is not expected to close the app and the file copy would then fail (inferred from the winit source, not yet observed on Windows). A forced close is covered by `recovery.sdw`, as with `Cmd+Q`. The workflow downloads Inno Setup from a pinned URL and checks its SHA-256 because the `windows-latest` image does not reliably include it; Dependabot does not track that pin, so bump URL and hash together by hand.
- `.cargo/config.toml` links the MSVC C runtime statically, so the installed exe does not need the Visual C++ Redistributable. `RUSTFLAGS` silently replaces it, so the release workflow checks the exe's imports with `dumpbin /dependents` and fails on `vcruntime`/`ucrtbase`/`api-ms-win-crt`.
- `main.rs` sets `windows_subsystem = "windows"` only without `debug_assertions`, so a Windows release build has no stderr. `report.rs` covers that gap: `report::run` shows a message box when `iced` returns an error or the main thread panics, and `report::warn` does the same for startup warnings such as an unsupported `language` file; elsewhere both print to stderr. Panics in library jobs are already caught in `work.rs` and reported in the UI, so the panic hook only records main-thread panics.
- `packaging/linux/package.sh <version>` changes to the repo root (so rustup picks `rust-toolchain.toml`), builds the release binary, and writes `<target dir>/dist/SolideoWorship-<version>-linux-<arch>.tar.gz`, with the target dir taken from `cargo metadata`; CI calls the same script on `ubuntu-22.04` for an older glibc baseline (move to the oldest remaining Ubuntu runner when it is retired). `install.sh` installs per user into `~/.local` and refuses to run as root. Its `Exec=` line is quoted and escaped per the Desktop Entry spec; a path containing `%` is refused because GLib looks up the program before expanding `%%`.
- `APP_ID` in `crates/app/src/windows.rs` must equal the basename of `packaging/linux/solideoworship.desktop` and its `StartupWMClass`; Wayland compositors match windows to the desktop entry (icon, window rules) through it.
- The logo lives in `assets/logo/`: the SVGs are the masters, and `render.sh` (needs `rsvg-convert` and `python3`) regenerates `png/` and `solideoworship.ico`; commit the outputs, CI does not render them. Icons below 48 px use dedicated cuts (`app-icon-16.svg`, `app-icon-24.svg`, and `app-icon-32.svg`, each drawn on whole pixels at its size) because the eight-line mark turns to grey mush there. The installer (`SetupIconFile`, wizard images), the exe icon (`crates/app/build.rs` through `winresource`, Windows hosts only), and the Linux archive (`package.sh` → hicolor icons) all read from it. `logo.rs` redraws the mark with iced containers for the splash and toolbar and rasterizes the 32 px window icon itself, so no image decoder or SVG feature is compiled in; its tests read the SVG masters, so a changed SVG fails `cargo test` until `logo.rs` follows. winit uses the window icon as the Windows small icon (iced has no setting for the large one, so the taskbar may scale it) and as `_NET_WM_ICON` on X11, and ignores it on Wayland and macOS, where the desktop entry or a future app bundle provides it.
- The UI fonts live in `assets/fonts/`: Inter (Regular, Medium, SemiBold, Bold) and a Lucide icon font, cut down by `subset.sh` (needs `curl`, `unzip`, `python3`, and `pyftsubset` from fonttools) from pinned, SHA-256-checked releases; commit the outputs, CI does not run it. `style::FONTS` embeds them with `include_bytes!` and `main` registers them before `default_font(Inter)`, so the projector renders the same face on Windows and macOS instead of each system's sans-serif. An icon is a `&str` constant in `icon.rs` drawn as text in the `lucide` family; to add one, put its Lucide name in `ICONS` in `subset.sh` and rerun it, which also prints the code points. Their licenses (OFL-1.1 and ISC) ship next to `LICENSE` through `package.sh` and the Inno Setup script.

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

### Internationalisation

UI copy never appears as a literal in code. It lives in `crates/app/locales/<code>.json`; `en.json` is the source of truth. `crates/app/build.rs` turns the locales into nested `Strings` structs of `&'static str` plus one static table per language, so views read `i18n::t().section.key` and `text(..)` stays `Fragment::Borrowed` (rule 3 below). An unknown key, a mismatched `{placeholder}`, or leading/trailing whitespace in a translation fails the build; a missing or empty value falls back to English with a `cargo:warning`. Keep separators and spacing in the layout code, not in translations, and put a label used in several places under `common` instead of duplicating it. Counts use `_one`/`_other` key pairs picked with `i18n::plural`. Placeholders are filled with `i18n::fill`, only where a `String` is built anyway (status messages, errors), never for static labels. Fields never read trigger `dead_code`, which flags unused keys. Keys are snake_case; Rust keywords are emitted as raw identifiers (`t().theme.r#type`).

The language is chosen once in `main` (`SDW_LANG`, then the `language`/`language.txt` file in the data dir, then, on Windows only, the same file next to the executable, both decoded from UTF-8 or UTF-16, then the OS locale via `sys-locale` — already compiled in through cosmic-text — then English) and stored in a `OnceLock`. The sources are consulted lazily in that order, so a later file is not even read once an earlier source decides. There is no in-app switcher by design — the Windows installer writes `{app}\language` — which is why `t()` can be a global instead of state threaded through every `view`. The installer writes next to the executable, not into the data dir, because an all-users install runs elevated and `%LOCALAPPDATA%` would then belong to the administrator, not the operator.

`sdw-core` and `sdw-library` stay free of i18n: their `Display` impls are English developer messages. The app maps their error variants to translated text through `i18n::Localize`, and enum labels through `i18n::Label` — both live in `i18n.rs`. The library error enums are deliberately not `#[non_exhaustive]`, so a new variant fails to compile until it is translated. Bible book names and abbreviations in `crates/core/src/bible.rs` are Indonesian *data* used by the reference parser, not UI copy; localizing them is deferred.

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
- Do not wrap interactive widgets in `responsive`. It rebuilds its content on every layout, so a hovered button inside loses its status and requests a redraw forever: measured at about 86 % CPU while idle with the cursor over a preview thumbnail, and keyboard input stopped arriving under Xvfb. The preview grid sizes its thumbnails from the control window width, tracked through `window::resize_events`. `responsive` around content that does not react to the cursor, such as `slide::stage`, is fine.
- `pick_list` shows one plain string per option, so the screen and schedule menus use `popover.rs`, a small overlay widget anchored under its button that closes on an outside click or Escape. It needs iced's `advanced` feature, which is a feature flag of the existing dependency, not a new crate.

Check the version resolved in `Cargo.lock` and read the crate source in `~/.cargo/registry` rather than assuming an API exists.

## Conventions

Code, identifiers, comments, test names, and docs are written in English. UI copy goes in the locale files (see Internationalisation), never in code. Follow the global comment rules — comments explain *why*, not *what*.

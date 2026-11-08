# SolideoWorship

A lightweight, fast, open-source worship presentation app (song lyrics and Bible verses), written in Rust.

SolideoWorship started from a simple problem: apps like EasyWorship use a lot of RAM and feel slow on low-end laptops, yet those are exactly the laptops church media operators use most. The goal of this project is a workflow that feels familiar to EasyWorship users, with **performance and memory efficiency as the top priority**.

> **Status: early development.** Every MVP feature (songs and Bible) is in place, but it has not yet been tested in a real service or on Windows and macOS.

## What works today

- **Two windows, one state.** An operator window on the laptop and a fullscreen output window on the projector. Both render from the same state, so the projector can never differ from what the operator sees.
- **EasyWorship-style operator layout.** Schedule, Preview, and Live side by side, with the Library panel (Songs/Bible tabs and search) below them.
- **Song library in SQLite.** Full-text search over titles and lyrics as you type, with word-prefix matching and diacritic tolerance. Lists only hold titles; full lyrics are read when a song is selected.
- **Song editor.** Add, edit, and delete songs with a slide preview that updates as you type. Edited songs are updated in the schedule too.
- **Bible.** Translations are imported from Zefania XML files and looked up with references such as `Yoh 3:16-18`, `1 Kor 13:4-7`, or `Mzm 23`. Book names currently use Indonesian names and abbreviations. Verses appear as you type the reference, and passages are split across slides without ever cutting a verse in half.
- **Theme settings.** One theme for songs and one for the Bible: background, text, and footer colors, font size, line spacing, margin, alignment, and line or character limits per slide. Changes show up immediately in the dialog preview. A new theme reaches the projector the next time an item goes live, so changing it mid-service does not surprise the congregation.
- **`.sdw` schedule files.** Song lyrics and verses are stored inside the file, so a schedule can be prepared at home and opened on the church laptop even with a different library. Unsaved changes are marked, and the app asks before discarding them. If the app is closed abruptly before saving (crash, power cut, or `Cmd+Q` on macOS), the schedule is restored automatically the next time the app starts.
- **Projector screen selection.** Detects every monitor and prefers external screens. The output can be moved to another screen, and if the output screen is no longer detected when the screen list is reloaded, the output window is closed.
- **Show / Black / Clear output states.** Clear hides the text but keeps the background.
- **Song compiler.** Sections are split automatically by the maximum lines per slide. Continuation chunks are numbered, e.g. "Verse 1 (2)".
- **Scripture compiler.** Verses are grouped per slide by a character limit without ever being cut in the middle, and the reference footer (e.g. "Yohanes 3:16-17 (TB)") follows the verses currently shown.
- **WYSIWYG.** Every theme size is expressed on a 1920×1080 reference canvas and then scaled, so thumbnails, the preview, and the projector share an identical layout.
- **Localized interface.** The UI is available in English and Indonesian (see [Language](#language)).

## Writing lyrics

Section headings go in square brackets on their own line. A blank line starts a new slide, and sections longer than the lines-per-slide limit are split automatically.

```
[Verse 1]
In Your name there is victory
In Your name there is restoration

Nothing is impossible for You
You are my hope

[Chorus]
You are my hope in the storm
You are my strength when I am weak
```

The example above produces three slides: "Verse 1", "Verse 1 (2)", and "Chorus".

## Importing a Bible

Bible texts are not bundled with the app because of their publishers' copyright (for example LAI for the Terjemahan Baru). Use a Zefania XML file you have the rights to:

1. Open the **Bible** tab in the Library panel, then click **Manage Bibles**.
2. Click **Choose file…** and pick a Zefania-format `.xml` file (`XMLBIBLE` → `BIBLEBOOK` → `CHAPTER` → `VERS`), UTF-8 encoded.
3. Check the abbreviation (filled in from the file name, e.g. `TB`), then click **Import**.

Footnotes and cross references inside the file are ignored, so they never reach the projector. A complete Bible (about 31 thousand verses) adds roughly 4 MB to the library database.

For books with a single chapter (Obaja, Filemon, 2 Yohanes, 3 Yohanes, Yudas), a single number is read as the verse number: `Yud 3` is the same as `Yud 1:3`.

References across chapters (e.g. `Yoh 3:16-4:2`) are not supported yet; add each chapter as a separate item.

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `→` / `Space` / `Page Down` | Next live slide |
| `←` / `Page Up` | Previous live slide |
| `Enter` | Send the preview slide to live |
| `B` | Black screen (press again to return) |
| `C` | Clear text (press again to return) |
| `F5` | Open the output window |
| `Esc` | Close the output window |
| `Enter` in the song search box | Add the top result to the schedule |
| `Enter` in the verse reference box | Add the passage to the schedule |
| `Ctrl+S` / `Cmd+S` | Save the schedule, or save the open dialog |
| `Ctrl+O` / `Cmd+O` | Open a schedule file |

While a dialog (song editor, theme settings) is open, operator shortcuts are disabled so typing never changes the projector screen.

## Language

The interface language is chosen once, at startup. There is deliberately no language switcher inside the app. The installer will write the choice, and until then it can be set by hand:

1. The `SDW_LANG` environment variable, e.g. `SDW_LANG=id`.
2. Otherwise, a plain-text file named `language` in the [data folder](#data-location) containing a language code such as `id` or `en`.
3. Otherwise, English.

Codes are case-insensitive and may carry a region (`id-ID`, `en_US.UTF-8`). Unknown codes fall back to English.

### Adding a translation

UI text lives in [`crates/app/locales`](crates/app/locales), one JSON file per language. `en.json` is the source of truth.

1. Copy `en.json` to `<code>.json` (a lowercase language code, e.g. `de.json`) and translate the values. Leave keys and `{placeholders}` unchanged.
2. Run `cargo build`. The build script turns every locale into a static table, so the new language is picked up automatically.

The build fails if a translation has a key that does not exist in `en.json` or uses different placeholders. A missing or empty value falls back to English with a build warning, so a partial translation still builds.

## Roadmap

The MVP scope is deliberately limited to **songs and Bible verses**.

- [x] Two-window skeleton and projector screen picker
- [x] Slide model, themes, and song and scripture compilers
- [x] Operator view: schedule, preview, live, library
- [x] Song library stored in SQLite with full-text search (FTS5)
- [x] Song editor (add, edit, delete)
- [x] Bible importer and chapter/verse lookup
- [x] Theme settings (colors, sizes, alignment, slide splitting)
- [x] Save and open schedules
- [x] Localized interface (English, Indonesian)

After the MVP: images and video as backgrounds, font selection, Bible book names per language, and importing from EasyWorship/OpenLP libraries.

## Performance principles

These principles were set from the start and are kept on purpose:

- The library is never loaded into memory in full. Song lists only hold titles, full lyrics are read when a song is selected, and verses are fetched per displayed range.
- Immutable text is stored as `Box<str>`/`Arc<str>`, and decks are shared through `Arc`. Sending a slide from Preview to Live copies no slide contents at all.
- `view` functions borrow data instead of cloning it, so redraws do not allocate new strings. Translated strings are `&'static str` from a table generated at compile time, so localization adds no allocation either.
- Reactive rendering: the screen is only redrawn when something changes, never every frame.
- Database access runs on its own worker thread, so searching or saving never blocks the operator's key presses.

## Platforms

| Platform | Status |
| --- | --- |
| Windows | Primary target |
| macOS | Primary target |
| Linux | Builds and runs, not yet a release target |

## Data location

The song library, Bible translations, and theme settings are stored in a single SQLite file, `library.sqlite3`:

| Platform | Folder |
| --- | --- |
| Windows | `%LOCALAPPDATA%\SolideoWorship` |
| macOS | `~/Library/Application Support/SolideoWorship` |
| Linux | `$XDG_DATA_HOME/solideoworship` (usually `~/.local/share/solideoworship`) |

The same folder holds `recovery.sdw` while the schedule has unsaved changes. It is deleted automatically once the schedule is saved. The optional `language` file described under [Language](#language) lives here too.

The `SDW_DATA_DIR` environment variable overrides that folder, which is useful for trying the app without touching your main library.

## Building from source

Requires [Rust](https://rustup.rs), plus a C compiler for the bundled SQLite: Visual Studio Build Tools on Windows, or the Xcode Command Line Tools (`xcode-select --install`) on macOS. The minimum Rust version is the `rust-version` in `Cargo.toml`. With rustup, the version CI uses (`rust-toolchain.toml`) is installed automatically on the first build.

On Linux, open/save dialogs use `xdg-desktop-portal`, or `zenity` when no portal is available.

```sh
git clone https://github.com/MikeNoppo/solideoworship-app.git
cd solideoworship-app
cargo run -p sdw-app
```

For an optimized release build:

```sh
cargo build --release -p sdw-app
```

The binary ends up in `target/release/sdw-app` (or `sdw-app.exe` on Windows).

## Project structure

```
crates/
├── core/     sdw-core    — domain types and pure logic (songs, verses, slides, themes, compilers, live navigation)
├── library/  sdw-library — SQLite storage (songs, Bibles, themes, migrations), Zefania importer, and the .sdw file format
└── app/      sdw-app     — the iced GUI application, including the locales and the i18n build script
```

`sdw-core` deliberately depends on neither iced nor SQLite, so domain logic can be tested without a GUI and changes there do not trigger a slow iced rebuild. Songs and verses both compile into a `SlideDeck`. The live engine and the renderer only know `Slide`, so a new content kind only needs a new compiler.

## Development

```sh
cargo check                  # fast type check
cargo test --workspace       # every test, including SQLite against an in-memory database
cargo clippy --all-targets   # lint
cargo fmt                    # format code
```

`cargo clippy` reports a *future-incompatibility* warning for `block v0.1.6`. It comes from a macOS dependency of `display-info`, not from this project's code.

Every push to `main` and every pull request is checked by CI. On Linux, Windows, and macOS: clippy with no warnings, the full test suite, and `cargo check` with the minimum Rust version. On Linux only: formatting, plus dependency licenses, sources, and security advisories through [cargo-deny](https://github.com/EmbarkStudios/cargo-deny). The advisories check does not block pull requests and also runs weekly. A new dependency must come from crates.io and use a GPL-3.0-compatible license; the list is in `deny.toml`. On Linux, building needs the `libxcb1-dev` package (or your distribution's equivalent).

Contributions are very welcome. Commits follow [Conventional Commits](https://www.conventionalcommits.org). Code, identifiers, comments, and test names are written in English; UI text goes in the locale files, never as literals in the code.

## License

Copyright © 2026 MikeNoppo.

SolideoWorship is released under the [GNU General Public License v3.0](LICENSE) (`GPL-3.0-only`).

You are free to use, study, change, and share this app, including for any church or ministry. Anyone who distributes it or a derived version, free or paid, must make the complete source code available under the same license and keep the copyright notice. Closed-source versions are not allowed.

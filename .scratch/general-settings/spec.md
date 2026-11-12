# General Settings v1

Status: approved for implementation by the conversation's final plan and subsequent request to continue.

## Outcome

Replace the current Theme dialog with a Settings dialog using the existing SolideoWorship visual design. Provide General, Output, Song and Scripture categories, a draft preview and Reset category / Cancel / Apply / OK actions. All preferences are global to this computer, persist between sessions and do not alter the schedule file format or schedule dirty state. The source requirement is the agreed General Settings plan in the conversation; this file records its implementation contract.

## Behavior

- General: open output at startup (default off), advance schedule preview after a successful Go Live (default off), UI panel shadows (default on).
- Output: remembered monitor, refresh monitors, 16:9 (default) or 4:3 canvas, four safe margins outside content padding. Canvas width is 1920 reference units; height follows aspect ratio. Remember monitor descriptors, never transient OS ids alone. Missing/ambiguous remembered monitors stay unavailable; do not auto-open on another screen.
- Song/Scripture: system font family and Normal/Medium/Semibold/Bold weight for body and footer separately, existing sizes/colors/alignment/line height/padding/slide limits. Unknown fonts resolve to bundled Inter and are identified in Settings. Font catalog is cached and searchable, bounded result lists.
- Song: optional section heading above lyrics (default off), copyright visibility (default on), optional global license text appended in footer. Visible copyright/license appear on each slide. Operator labels remain present regardless of projected heading visibility.
- Scripture: verse numbers, slide reference and translation visibility (all default on), character-boundary grouping or one complete verse per slide. Neither mode splits a verse. Keep full source text.
- Settings preview uses the selected source or existing sample sources. Draft edits affect this preview only. Expose safe-area guides and a warning when text does not fit, using renderer-equivalent text measurement, without resizing/splitting or disabling Go Live. No warnings/guides on congregation output.
- Apply saves atomically and stays open. OK saves and closes. Cancel/X/Escape discard edits since the last Apply. Saving prevents edits, resets, duplicate saves and closing. Failed saves retain the draft and show a translated error. Async completions use modal tickets.
- After successful save, operator preview uses the new settings; current live retains its deck, theme, canvas and margins until next Go Live. Monitor change is the exception: immediately close the old output then reopen on the chosen monitor, preserving content/index/theme/Black/Clear. Never have two output windows simultaneously.
- Advance schedule only after copying the selected preview into Live; at the last item stay there. Startup output opens after preferences/displays/control are ready, with no projected material before Go Live. Missing monitor prevents startup output.
- Existing toolbar monitor selection remains available and shares preference state with Settings. New output uses current settings when no presentation is live.

## Shared implementation contract

Core is GUI/SQLite/serde-free. Add `settings` module and export `Settings`, consisting of `styles: Styles`, `general: GeneralSettings`, `output: OutputSettings`. General has `open_output_on_startup`, `advance_after_go_live`, `shadows`. Output has `monitor: Option<MonitorPreference>` and `layout: OutputLayout`. MonitorPreference holds backend name, friendly name and resolution (string or physical dimensions), independent of iced.

`OutputLayout` has `format: CanvasFormat` (Wide / Standard), `margins: ScreenMargins` (left/right/top/bottom f32), and ratio/height/content-area methods. Add `layout: OutputLayout`, `body_font: FontSpec`, `footer_font: FontSpec`, `show_label: bool` to `SlideTheme`. Layout on theme is a derived rendering snapshot: `Settings::clamped()` synchronizes both themes from the canonical output layout; persist layout only under OutputSettings. This preserves the existing theme-only render interface and live_theme snapshot. Reject invalid effective content area at save; loaded invalid values use conservative defaults/clamping.

`FontSpec { family: Box<str>, weight: FontWeight }`, Normal/Medium/Semibold/Bold, default Inter/Normal. Add `show_copyright: bool`, `license: Box<str>` to SongStyle; add `show_verse_numbers`, `show_reference`, `show_translation`, `one_verse_per_slide` to ScriptureStyle. Keep old compiler entrypoints compatible; introduce style-aware entrypoints and route ScheduleItem::compile through them. Preserve operator labels, generic renderer and shared Arc decks.

Library exposes `settings()` / `save_settings(&Settings)` in the existing settings table. Existing styles/shadows are fallback for missing or broken new settings. Read partial fields using defaults. Save one aggregate JSON record atomically. Keep existing styles/shadows methods usable for backward compatibility and tests. Never edit shipped migrations.

App: extend the existing ThemeDialog module rather than duplicating it. Retain `ThemeDialog::new` and `Action::Save` as compatibility adapters until the integration ticket; add `ThemeDialog::settings(&Settings, current, displays)`, `Action::SaveSettings(Settings, bool)` where bool means close-after-save, `saved(&Settings)` to clear busy/rebase after Apply, and a monitor-refresh action. Existing Message::Save can mean OK; add Apply. Main integration will switch boot/persistence to Settings and rename user-visible Theme to Settings. Font/layout helpers live in a small app module accessed by renderer and dialog; use existing iced advanced graphics FontSystem and Paragraph, no new dependencies. Do not enumerate or shape on every view; cache by material/settings revision.

## Acceptance and validation

Existing preferences survive; all options affect real previews/compiler/output. Saving is atomic, live is stable except explicit monitor move, draft cancel never mutates runtime preferences, and schedule serialization is unchanged. Test through core compile/settings boundaries, Library::open_in_memory, dialog update/action and App update state. These are the approved test seams from the plan; no further permission needed.

Run cargo fmt --check, cargo test --workspace --profile ci, cargo clippy --all-targets --profile ci -- -D warnings and cargo +1.92 check --workspace --all-targets --profile ci. Verify runtime visually on available Linux surface; explicitly report Windows/macOS hardware validation unavailable if so.

No new code comments. Preserve existing comments. English identifiers/tests/docs; UI copy in en/id locales. Conventional commits with scope and lowercase imperative description, no AI attribution. Worktrees must be based on feat/general-settings, merge integration before reporting done. Commit each logical verified slice. Do not push, open PRs or publish issues.

## Deferred

Label color/hotkeys, transitions, logo mode, multi-output/foldback, measured pagination, alerts/intervals/remote control, alpha/custom-position output, media/video, per-item overrides and in-app language switching are not v1 implementation work.

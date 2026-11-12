# 02: Apply song and scripture options to compiled slides

Blocked by: 00
Status: resolved on feat/settings-content

Deliver style-aware song/scripture compilation from ../spec.md. Preserve old entrypoints, route ScheduleItem through full style settings, apply footer visibility/license and verse formatting/grouping, preserve operator labels. Update song editor, Bible preview/counts and schedule recompilation to use full styles and preserve/clamp preview selection. Avoid changing main.rs or renderer/dialog modules; add compatibility adapters if needed and note main wiring for ticket 05.

- [x] Content controls change material exactly as specified, keeping every verse intact.
- [x] Compiler, editor, Bible counts and schedule tests pass.
- [x] Changes committed and integration tip merged before handoff.

Verified after merging integration `2aa60e5`: `cargo fmt --check` and isolated `CARGO_TARGET_DIR=/tmp/sdw-target-02 cargo test --workspace --profile ci --quiet` pass (165 app, 76 core, 6 settings integration, 64 library tests). Red tests reproduced missing song footer preferences, scripture visibility/grouping, and full-style app APIs before implementation. Inverting the new song footer switch made its regression test fail; the source was restored and the workspace suite passed. The integration's unused `stage_with_guides` warning awaits ticket 04/05 wiring.

Ticket 05 wiring:

- Replace `Editor::new(origin, song, max_lines)` with `Editor::with_style(origin, song, &settings.styles.song)`.
- Replace `BibleTab::found(..., max_chars)` with `BibleTab::found_with_style(..., &settings.styles.scripture)` for async passage results, using current settings when the result arrives.
- Call `BibleTab::set_style(&settings.styles.scripture)` after a successful settings save to refresh an existing passage's count/status without losing the complete source or its added state.
- Keep `Schedule::recompile(&settings.styles)` on successful save. It already preserves/clamps the preview cursor and leaves schedule dirty state and existing shared live decks intact.
- `ScheduleItem::compile` now routes through `compile::song_with_style` / `scripture_with_style`; allocation-free counts use `scripture_len_with_style`. Legacy compiler entrypoints remain compatible.

Song footer semantics agreed with integration: `show_copyright = false` hides both copyright and license. When enabled, trimmed nonempty license text follows copyright with ` · `, or stands alone if copyright is absent. Operator song labels, scripture references and the Bible source verse list remain available independently of projected visibility.

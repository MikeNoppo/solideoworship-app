# 04: Configure label rules and transition choices in Settings

Blocked by: 00
Status: complete

Add Song Labels and Transitions categories to existing dialog, with translated EN/ID copy, bounded label table/chip samples, add/delete aliases/color/shortcut edits, validation and category resets. Add Cut/Fade and durations for slide/Black/Clear separately and Preview action/state seams for05. Preserve Apply/OK/Cancel/busy/v1 fields. UI04 owns theme_dialog files, locales and i18n; no main/message runtime edits. Renderer03 API can be used after it integrates; actual demo ticking/wiring belongs05. Coordinate Card additions with02 to keep building. Native minimum-size QA follows integration.

- [x] All new preferences are editable/validated without losing original v1 settings.
- [x] Draft/busy/reset/error tests pass; labels/transition forms remain usable at minimum size.
- [x] Preview API exposes sample material and stops on closing/reset; runtime demo implemented by05.

Implemented on `feat/settings-p2-ui` in `/tmp/sdw-p2-04`, based on integration `5acacf9` (labels and sampled renderer included). Label fields keep raw draft text and validate globally before Apply/OK; invalid hidden-category text cannot save stale valid values. The bounded full-width label table uses the shared operator chip helper. Transitions retain independent Fade durations in Cut and provide local two-section sample Playback with per-row Preview buttons. `ThemeDialog::has_animation()` and `advance_animation(Instant)` are ready for ticket 05's conditional native frame wiring; no main/message runtime edits belong to this ticket.

Red/green runs covered invalid label color, independent raw transition durations, and demo lifecycle actions. Validation: `cargo test --workspace --profile ci` passed (215 app tests, 76 core unit tests, 11 core integration tests, 66 library tests); `cargo clippy --all-targets --profile ci -- -D warnings` and `cargo fmt --check` passed. Native minimum-size EN/ID QA remains the integrated launch check assigned to the root task.

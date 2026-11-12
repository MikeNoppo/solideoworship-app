# 02: Apply song and scripture options to compiled slides

Blocked by: 00
Status: claimed by settings_content on feat/settings-content

Deliver style-aware song/scripture compilation from ../spec.md. Preserve old entrypoints, route ScheduleItem through full style settings, apply footer visibility/license and verse formatting/grouping, preserve operator labels. Update song editor, Bible preview/counts and schedule recompilation to use full styles and preserve/clamp preview selection. Avoid changing main.rs or renderer/dialog modules; add compatibility adapters if needed and note main wiring for ticket 05.

- [ ] Content controls change material exactly as specified, keeping every verse intact.
- [ ] Compiler, editor, Bible counts and schedule tests pass.
- [ ] Changes committed and integration tip merged before handoff.

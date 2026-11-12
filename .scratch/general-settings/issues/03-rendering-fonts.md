# 03: Render configurable canvas, margins and fonts with overflow diagnostics

Blocked by: 00
Status: ready-for-agent

Deliver shared rendering/layout and font helpers from ../spec.md. Renderer reads derived theme layout, resolved body/footer fonts and optional label. Propagate ratios into slide cards and preview/live/editor fixed aspect sizing with minimal edits. Cache a searchable font catalog using existing iced APIs, fallback to Inter, and provide accurate text bounds/overflow helpers using the same shaping/wrapping/line-height as rendering. Expose preview safe-area guides while output remains clean. Keep existing rendering API compatible. Coordinate helper signatures via a short note in spec.md or external handoff file. Do not change main.rs, theme_dialog.rs or locale files; ticket 04 consumes helpers and owns dialog UI. Changes to editor.rs should be limited to ratio sizing; compiler ticket may also touch it.

- [ ] 16:9/4:3 and four margins render consistently everywhere.
- [ ] Fonts resolve predictably, overflow is measured at reference canvas size.
- [ ] Meaningful tests/checks pass; changes committed and integration merged.

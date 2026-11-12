# 03: Render configurable canvas, margins and fonts with overflow diagnostics

Blocked by: 00
Status: resolved on feat/settings-rendering; integration foundation merged

Deliver shared rendering/layout and font helpers from ../spec.md. Renderer reads derived theme layout, resolved body/footer fonts and optional label. Propagate ratios into slide cards and preview/live/editor fixed aspect sizing with minimal edits. Cache a searchable font catalog using existing iced APIs, fallback to Inter, and provide accurate text bounds/overflow helpers using the same shaping/wrapping/line-height as rendering. Expose preview safe-area guides while output remains clean. Keep existing rendering API compatible. Coordinate helper signatures via a short note in spec.md or external handoff file. Do not change main.rs, theme_dialog.rs or locale files; ticket 04 consumes helpers and owns dialog UI. Changes to editor.rs should be limited to ratio sizing; compiler ticket may also touch it.

- [x] 16:9/4:3 and four margins render consistently everywhere.
- [x] Fonts resolve predictably, overflow is measured at reference canvas size.
- [x] Meaningful tests/checks pass; changes committed and integration merged.

Helper signatures and cache usage are recorded in [rendering-api.md](../rendering-api.md). Renderer, thumbnails, live frame and editor stage use the theme snapshot's canvas ratio. All content uses combined safe margins and theme padding. Guides are available only through the explicit Settings preview helper.

Validation: `cargo fmt --check`, `cargo test --workspace --profile ci` (161 app, 70 core, 5 core integration and 57 library tests) and `cargo +1.92 check --workspace --all-targets --profile ci` passed with an isolated target directory. An inverted heading-visibility condition made its regression test fail, then was restored. Clippy with denied warnings currently reports the new helpers awaiting ticket 04 consumers and the existing enlarged ThemeDialog action payload; ticket 04 owns those consumers/payloads, and root runs final integrated checks and Linux GUI QA. Windows/macOS hardware validation is unavailable in this worker.

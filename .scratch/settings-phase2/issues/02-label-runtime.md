# 02: Color operator labels and navigate live sections safely

Blocked by: 00
Status: resolved — feat/settings-p2-labels, merged foundation at 55b1187

Deliver operator-only colored label chips in normal/Settings thumbnails and live rows as practical, and Alt/Option letter cycling through live song sections. Respect input focus, captured events, modals, auto-repeat, stale focus results and existing B/C shortcuts. No projector color changes, source edits or auto-opening output. Saved rule changes refresh operator styling/caches. Own App main/message and panes changes; UI04 owns theme_dialog files, coordinate Card API changes. Expose navigation seams for05 transition wiring. Keep behavior tests at App/core boundary.

- [x] Labels color only operator UI and remain legible with current/live selection.
- [x] Real section navigation cycles correctly without firing while typing or on Scripture.
- [x] Tests/checks pass, no per-view normalization allocation, no shortcut regressions.

Implemented shared allocation-free operator label lookup and chips across preview/editor/Settings/live rows; central App::navigate_live seam and exact Alt/Option section shortcuts with focus, repeat, modal and stale-result guards.

Validation: cargo fmt --check; cargo test -p sdw-app --profile ci (198 passed); cargo clippy -p sdw-app --all-targets --profile ci -- -D warnings. Native/platform QA and full workspace/MSRV checks remain with final integration.

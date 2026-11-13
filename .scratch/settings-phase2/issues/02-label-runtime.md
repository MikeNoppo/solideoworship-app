# 02: Color operator labels and navigate live sections safely

Blocked by: 01
Status: ready-for-agent

Deliver operator-only colored label chips in normal/Settings thumbnails and live rows as practical, and Alt/Option letter cycling through live song sections. Respect input focus, captured events, modals, auto-repeat, stale focus results and existing B/C shortcuts. No projector color changes, source edits or auto-opening output. Saved rule changes refresh operator styling/caches. Own App main/message and panes changes; UI04 owns theme_dialog files, coordinate Card API changes. Expose navigation seams for05 transition wiring. Keep behavior tests at App/core boundary.

- [ ] Labels color only operator UI and remain legible with current/live selection.
- [ ] Real section navigation cycles correctly without firing while typing or on Scripture.
- [ ] Tests/checks pass, no per-view normalization allocation, no shortcut regressions.

# 05: Integrate startup, live snapshots, persistence and monitor preferences

Blocked by: 06
Status: implemented / awaiting review — feat/settings-app, runtime through 5557e84

Wire settings into boot, modal actions, Library worker, monitor toolbar, live snapshots, schedule advance and output lifecycle. Remember descriptors robustly, missing/ambiguous displays stay unavailable, Apply moves an open output without changing live material or Black/Clear. Startup must wait for control/preferences/displays and never silently use a different screen. Modal tickets protect newer dialogs. Full styles reach editor and Bible count consumers. Remove superseded compatibility UI paths as appropriate while preserving legacy storage readers. Update README user guidance. Test App event state transitions and all spec acceptance paths; run complete formatting, tests, clippy, MSRV and available runtime QA. Resolve this ticket only after code review fixes too.

The resolved [dialog contract](06-dialog-contract.md) provides the save/apply and monitor refresh boundary. Integration can proceed alongside the Settings UI in ticket 04; it does not depend on that ticket's complete layout or controls.

- [ ] All v1 behavior works end to end; legacy preferences/files preserved.
- [ ] Cross-layer state tests and complete checks pass.
- [ ] Documentation and validation evidence recorded; final review issues fixed.

Runtime commits `09258b2` and `5557e84` switch the app to aggregate `Settings`, save through the library worker, wait for preferences/displays/control before startup output, and preserve live deck/theme/layout snapshots across preference changes. Monitor changes close before reopening and preserve Live content and Black/Clear state. Settings callbacks use modal tickets, Apply rebases the open draft, Escape follows busy/cancel semantics, and editor/Bible consumers receive complete styles. The toolbar monitor picker shares persisted descriptors. README guidance is committed in `f2db6f9`.

The complete ticket 04 UI and updated integration prerequisites merged into this branch without product-code conflicts. The aggregate `App.settings` and all Settings forms remain intact. This ticket stays open until the required final review, review fixes, and remaining verification are complete.

Validation: the runtime worker reported 182 app tests passing before the complete UI merge. The integration pass ran `cargo test -p sdw-app --profile ci` after that merge: all 187 tests passed, including startup readiness/load ordering, unavailable/ambiguous monitors, single-window monitor moves, stale callbacks, Apply/cancel/failure, live snapshots, and schedule advance. Final root `cargo fmt --check`, `cargo test --workspace --profile ci` (333 tests), and `cargo check --workspace --all-targets --profile ci` passed after both integrations. Transitional unused legacy adapters and locale keys remain for the final review-fix pass; strict clippy is not yet clean. MSRV and visual/Windows/macOS hardware validation remain unverified.

# 05: Integrate startup, live snapshots, persistence and monitor preferences

Blocked by: 06
Status: resolved — integrated, reviewed and verified on feat/general-settings

Wire settings into boot, modal actions, Library worker, monitor toolbar, live snapshots, schedule advance and output lifecycle. Remember descriptors robustly, missing/ambiguous displays stay unavailable, Apply moves an open output without changing live material or Black/Clear. Startup must wait for control/preferences/displays and never silently use a different screen. Modal tickets protect newer dialogs. Full styles reach editor and Bible count consumers. Remove superseded compatibility UI paths as appropriate while preserving legacy storage readers. Update README user guidance. Test App event state transitions and all spec acceptance paths; run complete formatting, tests, clippy, MSRV and available runtime QA. Resolve this ticket only after code review fixes too.

The resolved [dialog contract](06-dialog-contract.md) provides the save/apply and monitor refresh boundary. Integration can proceed alongside the Settings UI in ticket 04; it does not depend on that ticket's complete layout or controls.

- [x] All v1 behavior works end to end; legacy preferences/files preserved.
- [x] Cross-layer state tests and complete checks pass.
- [x] Documentation and validation evidence recorded; final review issues fixed.

Runtime commits `09258b2` and `5557e84` switch the app to aggregate `Settings`, save through the library worker, wait for preferences/displays/control before startup output, and preserve live deck/theme/layout snapshots across preference changes. Monitor changes close before reopening and preserve Live content and Black/Clear state. Settings callbacks use modal tickets, Apply rebases the open draft, Escape follows busy/cancel semantics, and editor/Bible consumers receive complete styles. The toolbar monitor picker shares persisted descriptors. README guidance is committed in `f2db6f9`.

The complete ticket 04 UI and updated integration prerequisites merged without product-code conflicts. The aggregate `App.settings` and all Settings forms remain intact. Both review findings and the overflow-warning placement observation were resolved by ticket 07 and integrated at `6044ccd`.

Final root validation passed at `6044ccd`: formatting, all 336 workspace tests, strict all-target clippy and Rust 1.92 workspace/all-target check. The obsolete integration adapters and unused locale keys were removed without suppressions. Native Linux QA verified Apply/Cancel persistence, restart, both interface languages, canvas/margins, font selection/fallback, content options, startup output, Black/Clear, and unchanged Live output until the next Go Live. The final overflow-warning placement was checked at both normal and minimum window sizes. See [runtime-qa.md](../runtime-qa.md) for evidence and platform limits. Windows/macOS hardware and physical multi-monitor testing remain unperformed.

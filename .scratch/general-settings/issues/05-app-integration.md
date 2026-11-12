# 05: Integrate startup, live snapshots, persistence and monitor preferences

Blocked by: 06
Status: ready-for-agent

Wire settings into boot, modal actions, Library worker, monitor toolbar, live snapshots, schedule advance and output lifecycle. Remember descriptors robustly, missing/ambiguous displays stay unavailable, Apply moves an open output without changing live material or Black/Clear. Startup must wait for control/preferences/displays and never silently use a different screen. Modal tickets protect newer dialogs. Full styles reach editor and Bible count consumers. Remove superseded compatibility UI paths as appropriate while preserving legacy storage readers. Update README user guidance. Test App event state transitions and all spec acceptance paths; run complete formatting, tests, clippy, MSRV and available runtime QA. Resolve this ticket only after code review fixes too.

The resolved [dialog contract](06-dialog-contract.md) provides the save/apply and monitor refresh boundary. Integration can proceed alongside the Settings UI in ticket 04; it does not depend on that ticket's complete layout or controls.

- [ ] All v1 behavior works end to end; legacy preferences/files preserved.
- [ ] Cross-layer state tests and complete checks pass.
- [ ] Documentation and validation evidence recorded; final review issues fixed.

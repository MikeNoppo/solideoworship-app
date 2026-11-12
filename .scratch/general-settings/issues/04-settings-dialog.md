# 04: Edit preferences through the Settings dialog

Blocked by: 06
Status: resolved — feat/settings-dialog, implementation through 2f53c52

Extend existing Theme dialog into the complete four-category Settings UI specified in ../spec.md. Reuse existing style widgets/patterns; keep compatibility adapters for current main until 05. Localize all new user text in en/id. Provide monitor refresh/picker, searched body/footer font choices, toggles, limits, aspect/margins, safe-area preview and overflow diagnostics. Define draft/busy/Apply/OK/Cancel/category reset/saved/error semantics with behavioral tests. Maintain usable layout at 1120x720 app minimum. No interactive responsive wrappers. Do not modify main.rs/message.rs/toolbar; integration follows.

The dialog API through `26f8fe8` is integrated and recorded in [06-dialog-contract.md](06-dialog-contract.md). The complete UI is implemented through `2f53c52`; integration prerequisites were merged into this branch at `7d6b964`, preserving ticket 06 and the updated dependency graph. Ticket 05 wires the same contract independently.

- [x] Every v1 option is editable, localized and reflected in draft preview.
- [x] Save/cancel/reset/validation/error behavior passes public-action tests.
- [x] Workspace builds with old compatibility adapters; integrate with ticket 05 next.

The implementation adds General and Output forms, searched body/footer font selectors with bounded cached results, and Song/Scripture presentation controls. Category resets affect only the selected category. Invalid color or safe-margin input blocks saving, missing remembered monitors remain visible as unavailable, and stale monitor selections retain the saved descriptor. Draft preview rendering uses safe-area guides and cached overflow measurement without changing the source material.

Validation: the worker reported 22 dialog tests, 321 workspace tests, `cargo fmt --check`, and strict clippy passing before the prerequisite metadata merge. The integration pass reran all 22 dialog tests and `cargo fmt --check` successfully after that merge. Inverting the busy guard made the Apply regression test fail; the implementation was restored. Full integrated workspace checks follow ticket 05. MSRV and visual/hardware validation were not completed by this worker; final verification must report those separately.

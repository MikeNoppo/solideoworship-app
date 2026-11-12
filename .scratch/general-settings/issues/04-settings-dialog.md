# 04: Edit preferences through the Settings dialog

Blocked by: 06
Status: claimed — feat/settings-dialog, base c4a1aa7

Extend existing Theme dialog into the complete four-category Settings UI specified in ../spec.md. Reuse existing style widgets/patterns; keep compatibility adapters for current main until 05. Localize all new user text in en/id. Provide monitor refresh/picker, searched body/footer font choices, toggles, limits, aspect/margins, safe-area preview and overflow diagnostics. Define draft/busy/Apply/OK/Cancel/category reset/saved/error semantics with behavioral tests. Maintain usable layout at 1120x720 app minimum. No interactive responsive wrappers. Do not modify main.rs/message.rs/toolbar; integration follows.

The dialog API through `26f8fe8` is integrated and recorded in [06-dialog-contract.md](06-dialog-contract.md). This ticket remains claimed for the complete Settings UI; ticket 05 can wire the resolved contract concurrently.

- [ ] Every v1 option is editable, localized and reflected in draft preview.
- [ ] Save/cancel/reset/validation/error behavior passes public-action tests.
- [ ] Workspace builds with old compatibility adapters; commit and merge integration.

# 04: Configure label rules and transition choices in Settings

Blocked by: 01
Status: ready-for-agent

Add Song Labels and Transitions categories to existing dialog, with translated EN/ID copy, bounded label table/chip samples, add/delete aliases/color/shortcut edits, validation and category resets. Add Cut/Fade and durations for slide/Black/Clear separately and Preview action/state seams for05. Preserve Apply/OK/Cancel/busy/v1 fields. UI04 owns theme_dialog files, locales and i18n; no main/message runtime edits. Renderer03 API can be used after it integrates; actual demo ticking/wiring belongs05. Coordinate Card additions with02 to keep building. Native minimum-size QA follows integration.

- [ ] All new preferences are editable/validated without losing original v1 settings.
- [ ] Draft/busy/reset/error tests pass; labels/transition forms remain usable at minimum size.
- [ ] Preview API exposes sample material and stops on closing/reset; runtime demo implemented by05.

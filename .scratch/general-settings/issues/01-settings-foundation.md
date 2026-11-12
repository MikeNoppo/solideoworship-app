# 01: Persist complete settings with legacy compatibility

Blocked by: none
Status: ready-for-agent

Deliver the shared pure settings model and atomic Library settings boundary from ../spec.md. Extend styles with the agreed fields; keep all existing consumers building with conservative defaults. Test defaults, numeric/area validation, full preference round-trip, legacy/partial/broken data and failed writes preserving prior settings. Scope is core settings/theme types, library settings DTOs, and minimum struct initializer fixes needed to keep the workspace compiling. Compiler behavior is ticket 02. Do not implement dialog or renderer behavior.

- [ ] Shared contract implemented with legacy preferences preserved.
- [ ] Meaningful red/green tests and workspace check pass.
- [ ] Changes committed and integration tip merged before handoff.

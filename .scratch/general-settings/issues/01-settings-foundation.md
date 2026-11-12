# 01: Persist complete settings with legacy compatibility

Blocked by: 00
Status: claimed by settings_foundation (feat/settings-foundation)

Deliver the atomic Library settings boundary from ../spec.md using the shared core contract completed in ticket 00. Add settings() / save_settings(&Settings), persist one aggregate JSON record atomically, and retain existing styles/shadows methods and fallback behavior for missing or broken new settings. Read partial fields using defaults and preserve legacy preferences without changing shipped migrations. Test full preference round-trip, legacy/partial/broken data and failed writes preserving prior settings. Scope is library settings DTOs, storage and public in-memory Library tests. Do not implement compiler, dialog or renderer behavior.

- [ ] Aggregate settings storage implemented with legacy preferences preserved.
- [ ] Meaningful red/green tests and workspace check pass.
- [ ] Changes committed and integration tip merged before handoff.

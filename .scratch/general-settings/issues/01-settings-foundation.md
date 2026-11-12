# 01: Persist complete settings with legacy compatibility

Blocked by: 00
Status: resolved by settings_foundation (feat/settings-foundation)

Deliver the atomic Library settings boundary from ../spec.md using the shared core contract completed in ticket 00. Add settings() / save_settings(&Settings), persist one aggregate JSON record atomically, and retain existing styles/shadows methods and fallback behavior for missing or broken new settings. Read partial fields using defaults and preserve legacy preferences without changing shipped migrations. Test full preference round-trip, legacy/partial/broken data and failed writes preserving prior settings. Scope is library settings DTOs, storage and public in-memory Library tests. Do not implement compiler, dialog or renderer behavior.

- [x] Aggregate settings storage implemented with legacy preferences preserved.
- [x] Meaningful red/green tests and workspace check pass.
- [x] Changes committed and integration tip merged before handoff.

Verification: red/green tests exercised the public Settings and in-memory Library boundaries. `cargo test -p sdw-core -p sdw-library --profile ci` passed (140 tests), `cargo clippy -p sdw-core -p sdw-library --all-targets --profile ci -- -D warnings` passed, `cargo check --workspace --all-targets --profile ci` passed, `cargo +1.92 check -p sdw-core -p sdw-library --all-targets --profile ci` passed, and `cargo fmt --check` passed. Final checks used an isolated target directory after detecting shared worktree artifact overlap. Inverting monitor validation made the full settings round-trip test fail as expected, then the restored implementation passed.

The aggregate uses the existing settings table under the `settings` key. Layout is persisted only under OutputSettings; reads synchronize both theme snapshots. Missing fields inherit defaults or legacy styles/shadows, and malformed aggregate data falls back to legacy preferences. Validation failures return `Error::Io` with `InvalidInput`; failed validation or SQLite writes retain the previously saved aggregate and legacy keys. No migrations or schedule serialization changed. The integration tip was merged before handoff.

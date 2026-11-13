# 00: Define label rules, transition preferences and real song sections

Blocked by: none
Status: resolved

Integrated commit: 29bf1450edd64fd4c3a440f2b26f1b520965e764

Delivered the shared core contract in ../spec.md: label rules/validation/matching and defaults, independent transition preferences, song section starts and Presentation cycling behavior. Existing callers and legacy source/schedule APIs compile; minimal aggregate/i18n adapters preserve exhaustive matches pending persistence work in 01.

- [x] Nonempty heading blocks produce correct generic sections; continuations remain one section.
- [x] Aliases/numbered labels match and cycle; conflicting/invalid rules are rejected.
- [x] Root verification: `cargo test -p sdw-core --profile ci` passes all 87 tests.
- [x] Root verification: `cargo check --workspace --all-targets --profile ci` passes.

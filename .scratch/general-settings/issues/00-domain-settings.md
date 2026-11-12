# 00: Define the shared domain settings contract

Blocked by: none
Status: resolved

Deliver the pure core Settings contract from ../spec.md, including general/output preferences, remembered monitor descriptors, canvas and safe margins, font specifications, and song/scripture options. Keep core free of GUI, SQLite and serde dependencies. Preserve current behavior through conservative defaults, validate numeric values and effective content area, and synchronize both derived theme layouts from the canonical output layout when clamping loaded settings. Keep existing consumers building with minimum initializer fixes. Persistence is ticket 01; compiler and renderer behavior are tickets 02 and 03.

- [x] Shared core settings/theme contract implemented with existing behavior preserved by defaults.
- [x] Core defaults, numeric/area validation and layout synchronization tests pass.
- [x] Exact contract commit a41e07f integrated into feat/general-settings with a passing workspace check.

Validation: cargo test -p sdw-core --profile ci passed all 75 tests; cargo check --workspace --all-targets --profile ci passed.

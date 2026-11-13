# 01: Persist label and transition settings

Blocked by: 00
Status: resolved

Persist the shared contract delivered in 00 through aggregate settings storage, preserving v1 data. Keep existing callers compiling and legacy source/schedule APIs intact. The foundation implementer retains this ticket on feat/settings-p2-foundation; later commits require separate review before integration. No new UI or runtime wiring.

- [x] New preferences roundtrip and v1 records retain settings; targeted tests/workspace check pass.

Implementation: `e0a0189` (`feat(settings): persist label rules and transition choices`) on `feat/settings-p2-foundation`; shared contract: `29bf145`.

Validation: focused red/green tests for custom preference roundtrip, invalid-save rejection, and malformed-field fallback; `cargo test -p sdw-core -p sdw-library --profile ci` (87 core tests and 66 library tests passed); `cargo check --workspace --all-targets --profile ci`; `cargo clippy --all-targets --profile ci -- -D warnings`; `cargo fmt --check`. All use `/home/mikel/.cache/sdw-p2-target-01` for build outputs. Latest integration tracking merged before handoff.

Storage keeps its existing single aggregate-row write. Missing v2 fields default independently; malformed labels reset only labels and malformed transition specs reset only that choice. Duplicate loaded aliases/shortcuts reject the label section. No migration or schedule-file format changes.

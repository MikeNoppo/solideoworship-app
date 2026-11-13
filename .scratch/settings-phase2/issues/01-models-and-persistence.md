# 01: Persist label and transition settings

Blocked by: 00
Status: claimed

Persist the shared contract delivered in 00 through aggregate settings storage, preserving v1 data. Keep existing callers compiling and legacy source/schedule APIs intact. The foundation implementer retains this ticket on feat/settings-p2-foundation; later commits require separate review before integration. No new UI or runtime wiring.

- [ ] New preferences roundtrip and v1 records retain settings; targeted tests/workspace check pass.

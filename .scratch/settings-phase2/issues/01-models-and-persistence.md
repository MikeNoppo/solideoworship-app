# 01: Persist label/transition settings and compile real sections

Blocked by: none
Status: ready-for-agent

Deliver the core/shared contract in ../spec.md: label rules/validation/matching and defaults, transition settings, aggregate persistence preserving v1 data, song section starts and Presentation cycling behavior. Keep all existing callers compiling and legacy source/schedule APIs intact. Minimum i18n variant mapping fixes allowed to keep exhaustive matches compiling. No new UI or runtime wiring.

- [ ] Nonempty heading blocks produce correct generic sections; continuations remain one section.
- [ ] Aliases/numbered labels match and cycle; conflicting/invalid rules are rejected.
- [ ] New preferences roundtrip and v1 records retain settings; targeted tests/workspace check pass.

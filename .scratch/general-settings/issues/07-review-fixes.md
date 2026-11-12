# 07: Resolve final review findings and integration cleanup

Blocked by: two-axis review
Status: claimed — fix/settings-review

Resolve stale Settings monitor refreshes at the App event/state seam, centralize new localization mappings, and remove obsolete integration adapters and locale keys. Keep successful persisted save application, public core compiler entrypoints, and legacy Library style/shadow compatibility intact. Make the bounded font search test independent of host font ordering.

The verbatim final reviews and separate resolutions are recorded in [review.md](../review.md). Ticket 05 stays awaiting root final runtime QA and closure.

- [ ] Stale success/error callbacks preserve current runtime displays, selection, output and newer drafts.
- [ ] Localization follows Localize/Label and strict clippy has no integration dead code.
- [ ] Formatting, workspace tests, strict all-target clippy and Rust 1.92 all-target checks pass.

# 07: Resolve final review findings and integration cleanup

Blocked by: two-axis review
Status: resolved — fix/settings-review; code and automated checks complete, awaiting root integration and final GUI smoke

Resolve stale Settings monitor refreshes at the App event/state seam, centralize new localization mappings, and remove obsolete integration adapters and locale keys. Keep successful persisted save application, public core compiler entrypoints, and legacy Library style/shadow compatibility intact. Make the bounded font search test independent of host font ordering.

The verbatim final reviews and separate resolutions are recorded in [review.md](../review.md). Ticket 05 stays awaiting root final runtime QA and closure.

- [x] Stale success/error callbacks preserve current runtime displays, selection, output and newer drafts.
- [x] Localization follows Localize/Label and strict clippy has no integration dead code.
- [x] Formatting, workspace tests, strict all-target clippy and Rust 1.92 all-target checks pass.

Commits: `1631e8b` (stale refresh guard and regressions), `e0b2d58` (localization boundary), `c8c3a15` (transitional adapter and locale cleanup), `c4724d2` (font search test portability), and `5b86350` (overflow warning above canvas). The latter addresses the root GUI QA report without adding copy or redesigning the dialog.

Validation passed: formatting, 336 workspace tests, strict all-target clippy, Rust 1.92 workspace/all-target check, and diff whitespace check. Integration `c7bcfaf` was already included when merged before final reporting. Full evidence and separate review resolutions are in [review.md](../review.md). Ticket 05 remains awaiting final root QA; Windows/macOS hardware validation is unperformed.

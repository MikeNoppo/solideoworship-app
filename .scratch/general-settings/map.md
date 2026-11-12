# General Settings implementation

Tracker: local Markdown, one issue per file under issues/; mark claimed before work and resolved after integration and passing checks. The approved conversation plan is recorded in spec.md.

Integration branch: feat/general-settings

Graph: 00 -> (01, 02, 03); (01, 02, 03) -> 06 -> (04, 05); (04, 05) -> two-axis review -> review fixes -> final verification.

Ticket 00 is the completed pure domain contract. Storage (01), compiler behavior (02) and rendering/fonts (03) feed the dialog API contract (06), resolved through `26f8fe8`. The complete Settings UI (04) is resolved and integrated at `1b8fed0`. Runtime integration (05) is implemented and integrated at `7d34194`, awaiting the required final review and fixes before closure.

Test seams: pure core Settings/compile, Library public in-memory interface, dialog update/actions, App event/state boundary. Preserve existing tests and do not add implementation-mirroring tests.

## Decisions and evidence

- Local task tracking chosen to continue implementation without an external issue/PR publication step.
- Source spec is the user-reviewed Settings plan and request to proceed.
- Domain contract commit a41e07f is integrated; its 75 core tests and workspace all-target check pass with the ci profile.
- Dialog contract commits `b7b0f79`, `ca538d8`, `cc266e8` and `26f8fe8` are integrated by exact fast-forward. Ticket 06 records the API and focused verification; ticket 05 no longer waits for the full ticket 04 UI.
- The dialog and runtime branches merged without product-code conflicts. Integration preserved the four Settings categories, aggregate `App.settings`, live snapshots, and the shared monitor preference boundary. README guidance and ticket validation evidence are committed.
- Final root `cargo fmt --check`, `cargo test --workspace --profile ci`, and `cargo check --workspace --all-targets --profile ci` pass after both integrations: 333 tests total. Transitional unused legacy adapters/locale keys remain for the review-fix pass; MSRV and visual/hardware validation remain pending.

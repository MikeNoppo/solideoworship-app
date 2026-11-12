# General Settings implementation

Tracker: local Markdown, one issue per file under issues/; mark claimed before work and resolved after integration and passing checks. The approved conversation plan is recorded in spec.md.

Integration branch: feat/general-settings

Graph: 00 -> (01, 02, 03); (01, 02, 03) -> 04 -> 05 -> two-axis review -> review fixes -> final verification.

Ticket 00 is the completed pure domain contract. Storage (01), compiler behavior (02) and rendering/fonts (03) can proceed independently from that contract. The Settings dialog (04) waits for all three.

Test seams: pure core Settings/compile, Library public in-memory interface, dialog update/actions, App event/state boundary. Preserve existing tests and do not add implementation-mirroring tests.

## Decisions and evidence

- Local task tracking chosen to continue implementation without an external issue/PR publication step.
- Source spec is the user-reviewed Settings plan and request to proceed.
- Domain contract commit a41e07f is integrated; its 75 core tests and workspace all-target check pass with the ci profile.

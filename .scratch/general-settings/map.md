# General Settings implementation

Tracker: local Markdown, one issue per file under issues/; mark claimed before work and resolved after integration and passing checks. The approved conversation plan is recorded in spec.md.

Integration branch: feat/general-settings

Graph: 01 -> (02, 03) -> 04 -> 05 -> two-axis review -> review fixes -> final verification.

Test seams: pure core Settings/compile, Library public in-memory interface, dialog update/actions, App event/state boundary. Preserve existing tests and do not add implementation-mirroring tests.

## Decisions and evidence

- Local task tracking chosen to continue implementation without an external issue/PR publication step.
- Source spec is the user-reviewed Settings plan and request to proceed.

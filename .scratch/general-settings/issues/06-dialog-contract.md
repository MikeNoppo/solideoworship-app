# 06: Establish the dialog API contract for concurrent consumers

Blocked by: 01, 02, 03
Status: resolved

The completed API boundary from `feat/settings-dialog` is integrated through `26f8fe8` by exact fast-forward from `c4a1aa7`. Separating this boundary from the complete Settings UI lets tickets 04 and 05 proceed concurrently. Ticket 04 remains claimed; ticket 05 is ready for app wiring. Both depend on 06.

Integrated commits:

- `b7b0f79`: dialog drafts, validation, save/apply/busy callbacks and boxed action payloads.
- `ca538d8`: monitor descriptors, `Display::preference()` and conservative `display::matching()`.
- `cc266e8`: draft-preserving monitor refresh actions/callbacks and Settings locale strings.
- `26f8fe8`: reject a different monitor on a reused connector and avoid empty backend identifiers as exact matches.

Public app boundary:

- `ThemeDialog::settings(&Settings, Option<&ScheduleItem>, &[Display])` creates an independent settings draft. `ThemeDialog::new` and `Action::Save(Box<Styles>, bool)` retain the existing theme path until ticket 05 replaces its callers.
- `Message::Apply` returns `Action::SaveSettings(Box<Settings>, false)`; `Message::Save` returns the same action with `true` to request close after success. Both validate complete hex colors and settings before entering the busy state. While busy, updates return `Action::None`.
- `saved(&Settings)` accepts the successfully persisted settings, clears busy/error state and allows editing again. `fail(&str)` clears a failed save's busy state while retaining the draft for retry. Cancel returns `Action::Close` when no save is running.
- `Message::RefreshDisplays` returns `Action::RefreshDisplays` once while a refresh is pending. `set_displays(&[Display])` and `refresh_failed(&str)` finish refresh without replacing unsaved settings. Async results still require the modal ticket checks owned by ticket 05.
- `Display` includes `backend_name`, `physical_width` and `physical_height`. `preference()` produces the persisted `MonitorPreference`; `matching()` requires equal physical dimensions and returns a unique backend/friendly match, then a unique friendly fallback. Missing or ambiguous matches return `None`.

The minimal `main.rs` compatibility adaptation only handles the boxed legacy styles payload and leaves the new settings/refresh actions for ticket 05. This boundary does not resolve the four-category Settings UI or its remaining control coverage.

- [x] Exact contract commits integrated without later UI commits or worktree changes.
- [x] Public dialog and monitor behavior verified.
- [x] Workspace all-target check passes.

Validation: `cargo test -p sdw-app --profile ci theme_dialog::tests` passed (17 tests), `cargo test -p sdw-app --profile ci display::tests` passed (2 tests), and `cargo check --workspace --all-targets --profile ci` passed. The check reports unused monitor/font/overflow/refresh helpers awaiting ticket 04/05 consumers. Inverting the friendly-name guard made the reused-connector regression test fail by selecting the replacement screen; the implementation was restored and the monitor tests passed again.

# General Settings review

## Standards

Reviewed `git diff main...7d341944b11b211fa028c527e1e9ec9054832ac0` against AGENTS.md, CONTRIBUTING.md, CLAUDE.md, and the user’s stricter rules.

- **[P3] Follow the documented localization boundary.** `crates/app/src/theme_dialog.rs:508–514` translates `SettingsError` with `match error { SettingsError::InvalidValues => t().settings.invalid_values, ... }`. New enum labels are also mapped inside `theme_dialog/form.rs:133–136,249–254,284–287,354–359` and `theme_dialog/view.rs:62–67`. AGENTS.md, **Internationalisation**, specifies that the app maps error variants through `i18n::Localize` and enum labels through `i18n::Label`, both in `i18n.rs`. Add the corresponding implementations there and reuse them from the dialog. This is a documented convention breach, with no observed runtime failure.

No material baseline-smell findings. The diff adds no code comments or dependencies, preserves existing comments, keeps rendering text borrowed, and retains the domain/library boundaries. Commit subjects follow scoped Conventional Commits and inspected messages contain no AI attribution. Known legacy-adapter/obsolete-key cleanup was excluded as requested. This review did not independently build every historical commit.

**Count:** one low-priority documented-standard finding; zero material smell findings.

## Spec

# Spec review

Reviewed `main...7d341944b11b211fa028c527e1e9ec9054832ac0` against `.scratch/general-settings/spec.md` and local issue clarifications.

- **[P2] Reject stale monitor refreshes before updating runtime display state** — `crates/app/src/main.rs:741–748`. Spec line 17 requires: “Async completions use modal tickets.” The ticket guards only `dialog.set_displays`/`refresh_failed`; every completion still runs `DisplaysLoaded`. Trigger: start refresh in Settings A, cancel it, obtain a newer successful monitor refresh and select/open its monitor, then receive A’s older success/error. The old result overwrites the newer runtime list (`DisplaysLoaded(Err)` also clears `selected`), and an older list missing the active display closes that output through `apply_displays`. A newer Settings dialog retains its own available-monitor list, so its UI can also disagree with runtime. `work::background` starts independent threads, so refresh completion order is unconstrained. Ignore stale modal results before forwarding, or version all display enumerations so an older snapshot cannot supersede a newer one; cover the out-of-order success and error paths.

Missing/partial requirements: the async-ticket protection above is incomplete. No other confirmed omissions found in the reviewed v1 controls, persistence, compilation, startup gating, schedule advance, or live snapshot paths.

Scope creep: none found. The footer/license coupling follows the explicit issue-02 clarification. Transitional adapters/locale cleanup and hardware/runtime QA are excluded as requested. Review was read-only; no repo files changed.

## Resolutions

- P2 resolved in `1631e8b`: `SettingsDisplaysLoaded` now rejects the entire completion unless its ticket belongs to the active Settings dialog. Canceled or older refresh success/error cannot overwrite runtime displays, clear the selected monitor, close the output, or change a newer draft. Successful persisted `SettingsSaved` application remains global. The canceled-refresh regression failed against the old handler before the guard was added; the updated App event tests cover cancellation and out-of-order completion after a newer refresh, explicit monitor selection and output move, preserving the live deck/theme and Black state.
- P3 resolved in `e0b2d58`: `SettingsError` uses `i18n::Localize`; Category, CanvasFormat, FontWeight, MarginSide and FontSlot use `i18n::Label`. The dialog reuses those mappings with its existing captions.
- Integration cleanup resolved in `c8c3a15`: removed `Editor::new`, `BibleTab::found`, `ThemeDialog::new`/`kind`, `Action::Save` and the legacy flag, migrated tests to complete Settings/style interfaces, and deleted obsolete theme/toolbar locale keys in English and Indonesian. Save failure copy now exclusively uses the existing Settings wording. Public core compiler entrypoints and Library styles/shadows compatibility are unchanged. No lint suppressions were added.
- Font search portability resolved in `c4724d2`: the search test checks case-insensitive filtering and the requested bound without assuming bundled Inter sorts before every installed matching family.
- Root GUI QA finding addressed in `5b86350`: the existing overflow warning is now directly below the preview header/tabs and above the canvas, so a tall 4:3 preview cannot place it below the fold. Root's reproduction screenshots are `/tmp/sdw-settings-overflow.png` and `/tmp/sdw-settings-overflow-warning.png`. Final visual smoke confirmation remains with root.

## Validation

On `fix/settings-review`, with integration `c7bcfaf` already included and `CARGO_TARGET_DIR=/home/mikel/.cache/sdw-settings-target-05`:

- `cargo fmt --check`: passed.
- `cargo test --workspace --profile ci`: passed, 336 tests (190 app, 76 core, 6 core integration, 64 library), plus doc tests; zero failures, ignored tests or filtered tests.
- `cargo clippy --all-targets --profile ci -- -D warnings`: passed without integration dead-code warnings or suppressions.
- `cargo +1.92 check --workspace --all-targets --profile ci`: passed.
- `git diff feat/general-settings...HEAD --check`: passed.

Ticket [07](issues/07-review-fixes.md) is resolved for code and automated checks. Ticket [05](issues/05-app-integration.md) stays awaiting final root GUI QA and closure. This worktree did not perform Windows/macOS hardware validation, merge into integration, push, publish issues or open a PR.

## Final root closure

The worker handoff above is superseded by final integration and QA. Both review findings are resolved in integration `6044ccd`; root repeated all automated checks successfully and rebuilt the binary. Native Linux smoke confirmed the warning above the canvas at normal/minimum sizes. Tickets 05 and 07 are resolved. See [runtime-qa.md](runtime-qa.md) for observations and platform limits; Windows/macOS hardware validation remains unperformed.

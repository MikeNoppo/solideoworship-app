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

Pending in ticket [07](issues/07-review-fixes.md).

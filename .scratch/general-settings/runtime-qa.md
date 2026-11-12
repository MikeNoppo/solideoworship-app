# Settings v1 final verification

Reviewed product code: `6044ccd` on `feat/general-settings`. The final binary was rebuilt after the review fixes. Tests used a separate Xvfb display (`:108`, 1600x1000) and disposable `SDW_DATA_DIR` folders under `/tmp`; the user's application data was not changed.

## Automated checks

- `cargo fmt --check`: passed.
- `cargo test --workspace --profile ci`: 336 passed (190 app, 76 core, 6 core integration, 64 library), none failed or ignored.
- `cargo clippy --all-targets --profile ci -- -D warnings`: passed, with no suppressions.
- `cargo +1.92 check --workspace --all-targets --profile ci`: passed.
- Diff whitespace, code-comment preservation, commit conventions/attribution and unchanged dependency manifests/lockfile checks: passed.

## Native Linux observations

- All four categories render in English and Indonesian using the existing application design. The form and preview scroll independently; dialog actions remain accessible at the 1120x720 minimum control-window size.
- Apply saved a general preference and kept the dialog open. A subsequent edit followed by Cancel was discarded; reopening retained the applied value. Restarting the process retained preferences and the 4:3 canvas with left/top safe margins of 30/20 reference pixels.
- Font search returned bounded matches and an empty-result state. Selecting Adwaita Sans/Bold updated the preview and persisted the family/weight. A separate fixture with an unavailable saved family showed the translated fallback notice and rendered with bundled Inter.
- Projected section headings and license text appeared in the preview/output. Scripture toggles removed verse numbers/reference/translation, while one-verse mode preserved the complete verse and changed the draft preview grouping.
- The real fullscreen output rendered the configured 4:3 canvas with black side bars and no preview guides. Black produced exactly one black pixel color; Clear produced only the background and side-bar colors, without text.
- A live slide was captured, its song settings were reset/applied, and output was reopened. Comparing the before/after output screenshots produced zero differing pixels. The next Go Live switched to the new font/footer/heading settings.
- Startup with a stored available monitor opened exactly one output window with no projected material. Escape closed it without reopening it automatically.
- A long source triggered the overflow warning. After the final placement fix, the warning is visible above the canvas immediately at both 1480x920 and 1120x720; data remains intact and no automatic font shrinking/splitting was introduced.
- A one-second idle smoke sample consumed zero process CPU ticks after rendering settled. This is a reactive-rendering smoke check, not a cross-platform performance benchmark.

Temporary screenshot evidence includes `/tmp/sdw-settings-general.png`, `/tmp/sdw-settings-minimum-output.png`, `/tmp/sdw-settings-font-fallback.png`, `/tmp/sdw-settings-live-output.png`, `/tmp/sdw-settings-live-retained.png`, `/tmp/sdw-settings-live-new-settings.png`, `/tmp/sdw-settings-startup-output.png`, `/tmp/sdw-settings-final-overflow.png`, and `/tmp/sdw-settings-final-minimum.png`.

## Limits

Windows/macOS hardware, physical projectors and physical multi-monitor transitions were not available. Monitor identity/lifecycle, rapid moves, Black/Clear preservation and stale callback handling are covered by App event/state tests with synthetic display descriptors. Linux native QA used one virtual monitor. No Windows/macOS hardware validation or broad performance result is claimed.

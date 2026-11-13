# 05: Synchronize live, projector and Settings demo animations

Blocked by: 01,02,03,04
Status: implemented — pending independent Standards/Spec review and root QA

Integrate Scene/Playback into App live mutations including Go Live, arrows/row clicks/label shortcuts and Show/Black/Clear, using saved per-kind options. One shared frame sample drives live preview/output. Subscribe to native frame events only while animation or dialog demo exists; drop old snapshots and stop on completion. Retarget rapid inputs, preserve v1 monitor-close/move/Black/Clear and stale modal protections. Wire Settings Preview locally without changing real Live. Update README and record all checks/runtime QA. Remove interim adapters/unused keys without suppressions. Mark implemented pending review, not finally resolved until review fixes and root QA.

- [x] All navigation paths and modes use correct settings; latest input isn't queued behind animation.
- [x] Native frames stop at idle; preview/output/demo are consistent and snapshots bounded.
- [ ] Full tests/clippy/MSRV/format and available native QA pass; no v1 regressions.

Implementation: `cc842b7` and `ed1a007` on `feat/settings-p2-integration` in `/tmp/sdw-p2-05`. App owns one optional live Playback; both projector and LivePane borrow its cached frame. Go Live, row navigation, existing keyboard steps, and confirmed section shortcuts retarget immediately using saved slide options; explicit modes and B/C toggles select destination/departing-mode options. Window lifecycle cuts drop animation snapshots while preserving the existing logical move/Black/Clear rules. No extra keyboard bindings were introduced.

`AnimationFrame(Instant)` advances live and Settings demo from one conditional `window::frames()` subscription. Older timestamps are ignored by Playback; completion removes live snapshots and the frame subscription. Animation messages skip caption and recovery refresh. Settings demo stays local and stops through existing reset/save/cancel/close behavior. Static ready output continues to pick up applied background settings.

Focused App tests cover all those seams, duplicate targets, rapid Go Live/section navigation, shared demo/live frame lifecycle, no per-frame caption refresh, and Apply preserving the active Fade duration and v1 live theme snapshot. `cargo test --workspace --profile ci` passes: 224 app, 87 core, 66 library. `cargo fmt --check` and `cargo clippy --all-targets --profile ci -- -D warnings` pass. Dedicated build target: `/home/mikel/.cache/sdw-p2-target-05`. Native QA and final MSRV verification are assigned to root after integration; Windows/macOS hardware has not been tested here.

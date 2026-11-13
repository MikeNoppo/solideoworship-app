# 03: Sample and render bounded interruptible Fade transitions

Blocked by: 00
Status: implemented — integrated, pending independent review and root runtime QA

Deliver reusable Scene/Playback/sampled-frame rendering under render::transition plus slide alpha/background helpers, following ../spec.md and /tmp/sdw-phase2-fade-research.md. No main.rs/message.rs/theme_dialog/locales edits. Use theme borrow/Arc sharing. Publish a concise helper API pointer early for05. Deterministic time-based tests must cover endpoints, fade-out/in, direct mode fades, continuity on retargeting, backwards timestamps, completion cleanup and layout Cut fallback. No screenshot/GPU readback/new dependencies or permanent animation subscription.

- [x] Renderers consume a borrowed sampled frame and don't allocate theme/body strings per draw.
- [x] Interrupted transitions stay bounded and continuous; nonanimated endpoints are exact.
- [x] Focused tests/workspace checks pass; integration wiring remains05.

Implementation: `e26d367`, merged through `05e383d`; API documented in `../rendering-api.md`. Source/target snapshots share Arc decks, sampled frames borrow slide/theme data, interruption rebases visible opacity/background, and incompatible layouts/margins cut. The original status was stale after the renderer merge.

Validation: 16 focused renderer tests plus integrated workspace tests (224 app, 87 core, 66 library) pass on ticket 05. Full native EN/ID playback checks and independent Standards/Spec review remain with the root task.

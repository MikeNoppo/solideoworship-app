# 03: Sample and render bounded interruptible Fade transitions

Blocked by: 00
Status: ready-for-agent

Deliver reusable Scene/Playback/sampled-frame rendering under render::transition plus slide alpha/background helpers, following ../spec.md and /tmp/sdw-phase2-fade-research.md. No main.rs/message.rs/theme_dialog/locales edits. Use theme borrow/Arc sharing. Publish a concise helper API pointer early for05. Deterministic time-based tests must cover endpoints, fade-out/in, direct mode fades, continuity on retargeting, backwards timestamps, completion cleanup and layout Cut fallback. No screenshot/GPU readback/new dependencies or permanent animation subscription.

- [ ] Renderers consume a borrowed sampled frame and don't allocate theme/body strings per draw.
- [ ] Interrupted transitions stay bounded and continuous; nonanimated endpoints are exact.
- [ ] Focused tests/workspace checks pass; integration wiring remains05.

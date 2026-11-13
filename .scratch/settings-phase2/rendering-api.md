# Transition renderer API

Ticket 03 implementation: `feat/settings-p2-renderer`, commit `e26d367`, worktree `/tmp/sdw-p2-03`.

Module: `crates/app/src/render/transition.rs` (`render::transition`).

- `Scene::new(presentation: Option<Presentation>, theme: SlideTheme, state: OutputState) -> Scene`. All three fields are public. Presentation cloning shares the Arc deck; theme cloning happens only when constructing a command snapshot or interrupting, never during frame rendering.
- `Playback::new(scene: Scene) -> Playback` starts at an exact static endpoint.
- `Playback::retarget(scene: Scene, spec: TransitionSpec, now: Instant) -> bool` samples ongoing playback at the supplied time, rebases onto the visible body/opacity/background, and immediately installs the new logical target. Returns false for an identical target; that does not restart its existing fade. Different layout or theme margin forces Cut. An older command timestamp rebases from the latest accepted frame timestamp.
- `Playback::advance(now: Instant) -> bool` advances accepted timestamps and removes the source snapshot at completion. Returns false while idle or for duplicate/older timestamps. Call from the shared conditional native frame message; `frame()` never reads a clock.
- `Playback::is_active() -> bool` gates the native frame subscription.
- `Playback::target() -> &Scene` always describes the newest logical destination, including during animation.
- `Playback::frame() -> Frame<'_>` borrows whichever scene currently renders. A completed playback continues to return its exact target frame.
- `Frame::new(slide: Option<&Slide>, theme: &SlideTheme, state: OutputState) -> Frame<'_>` provides the same borrowed interface for a static scene/fallback.
- `Frame` is Copy with public `slide: Option<&Slide>`, `theme: &SlideTheme`, `body_opacity: f32`, `background: Rgb`.
- `transition::stage<'a, Message: 'a>(frame: Frame<'a>) -> iced::Element<'a, Message>` renders a borrowed frame. Suitable for output, LivePane and Settings demo. Wrap/size it just like existing `slide::stage`.
- `slide::stage_sampled(slide, theme, body_opacity, background)` is the lower-level borrowed helper; `slide::color_with_opacity` validates finite alpha and applies it uniformly to label/body/footer.

Different slides use outgoing text over the first half, then incoming text over the second half, with background interpolation over the whole duration. Same-scene mode changes interpolate body opacity directly over the entire duration. A hidden old body fades the incoming body in immediately. Playback retains only a source and target snapshot; interruption replaces both rather than appending history.

Existing `slide::stage`, `stage_with_guides`, `thumbnail`, and `output::view` remain usable. `output::view` uses the shared stage helper for static frames. Ticket 05 owns the real App state, output/LivePane sampled-frame selection and native frame subscription; ticket 04 can own an independent local Playback for its demo.

Validation before this commit: 16 focused render tests pass (`cargo test -p sdw-app --profile ci render::`, dedicated target directory). Full workspace checks follow. Unconsumed Playback methods cause dead_code warnings in the ordinary binary until ticket 04/05 integrates them; no suppressions were added.

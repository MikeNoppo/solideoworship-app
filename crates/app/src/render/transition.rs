use std::time::{Duration, Instant};

use sdw_core::slide::Slide;
use sdw_core::theme::{Rgb, SlideTheme};
use sdw_core::transition::{TransitionMode, TransitionSpec};
use sdw_core::{OutputState, Presentation};

pub fn stage<'a, Message: 'a>(frame: Frame<'a>) -> iced::Element<'a, Message> {
    super::slide::stage_sampled(
        frame.slide,
        frame.theme,
        frame.body_opacity,
        frame.background,
    )
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub presentation: Option<Presentation>,
    pub theme: SlideTheme,
    pub state: OutputState,
}

impl Scene {
    pub fn new(presentation: Option<Presentation>, theme: SlideTheme, state: OutputState) -> Self {
        Self {
            presentation,
            theme,
            state,
        }
    }

    fn same_content(&self, other: &Self) -> bool {
        self.theme == other.theme
            && match (&self.presentation, &other.presentation) {
                (Some(left), Some(right)) => {
                    std::ptr::eq(left.deck(), right.deck()) && left.index() == right.index()
                }
                (None, None) => true,
                _ => false,
            }
    }

    fn frame(&self) -> Frame<'_> {
        Frame::new(
            self.presentation.as_ref().map(Presentation::current),
            &self.theme,
            self.state,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    pub slide: Option<&'a Slide>,
    pub theme: &'a SlideTheme,
    pub body_opacity: f32,
    pub background: Rgb,
}

impl<'a> Frame<'a> {
    pub fn new(slide: Option<&'a Slide>, theme: &'a SlideTheme, state: OutputState) -> Self {
        Self {
            slide,
            theme,
            body_opacity: if state.shows_content() && slide.is_some() {
                1.0
            } else {
                0.0
            },
            background: if state.shows_background() {
                theme.background
            } else {
                Rgb::new(0.0, 0.0, 0.0)
            },
        }
    }
}

#[derive(Debug)]
pub struct Playback {
    target: Scene,
    transition: Option<Transition>,
}

#[derive(Debug)]
struct Transition {
    source: Scene,
    start: Instant,
    last: Instant,
    duration: Duration,
    progress: f32,
    direct: bool,
    source_opacity: f32,
    source_background: Rgb,
}

impl Playback {
    pub fn new(scene: Scene) -> Self {
        Self {
            target: scene,
            transition: None,
        }
    }

    pub fn retarget(&mut self, scene: Scene, spec: TransitionSpec, now: Instant) -> bool {
        self.advance(now);
        if self.target.state == scene.state && self.target.same_content(&scene) {
            return false;
        }
        let now = self
            .transition
            .as_ref()
            .map_or(now, |transition| now.max(transition.last));
        let frame = self.frame();
        let source_opacity = frame.body_opacity;
        let source_background = frame.background;
        let source = match &self.transition {
            Some(transition) if !transition.direct && transition.progress < 0.5 => {
                transition.source.clone()
            }
            _ => self.target.clone(),
        };
        self.transition = None;
        self.target = scene;
        self.transition = (spec.mode == TransitionMode::Fade
            && spec.duration_ms > 0
            && source.theme.layout == self.target.theme.layout
            && source.theme.margin == self.target.theme.margin)
            .then_some(Transition {
                direct: source.same_content(&self.target) || source_opacity == 0.0,
                source,
                source_opacity,
                source_background,
                start: now,
                last: now,
                duration: Duration::from_millis(u64::from(spec.duration_ms)),
                progress: 0.0,
            });
        true
    }

    pub fn advance(&mut self, now: Instant) -> bool {
        let Some(transition) = self.transition.as_mut() else {
            return false;
        };
        if now <= transition.last {
            return false;
        }
        transition.last = now;
        transition.progress = (now
            .saturating_duration_since(transition.start)
            .as_secs_f32()
            / transition.duration.as_secs_f32())
        .clamp(0.0, 1.0);
        if transition.progress == 1.0 {
            self.transition = None;
        }
        true
    }

    pub fn target(&self) -> &Scene {
        &self.target
    }

    pub fn is_active(&self) -> bool {
        self.transition.is_some()
    }

    pub fn frame(&self) -> Frame<'_> {
        let Some(transition) = &self.transition else {
            return self.target.frame();
        };
        let progress = transition.progress;
        let mut frame = if transition.direct {
            let target = self.target.frame();
            Frame {
                body_opacity: transition.source_opacity
                    + (target.body_opacity - transition.source_opacity) * progress,
                ..target
            }
        } else {
            let mut frame = if progress < 0.5 {
                transition.source.frame()
            } else {
                self.target.frame()
            };
            frame.body_opacity = if progress < 0.5 {
                transition.source_opacity * (1.0 - progress * 2.0)
            } else {
                frame.body_opacity * (progress * 2.0 - 1.0)
            };
            frame
        };
        let source = transition.source_background;
        let target = self.target.frame().background;
        frame.background = Rgb::new(
            source.r + (target.r - source.r) * progress,
            source.g + (target.g - source.g) * progress,
            source.b + (target.b - source.b) * progress,
        );
        frame
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use sdw_core::slide::{ContentKind, Slide, SlideDeck};
    use sdw_core::theme::{Rgb, SlideTheme};
    use sdw_core::transition::{TransitionMode, TransitionSpec};
    use sdw_core::{OutputState, Presentation};

    use super::{Playback, Scene};

    fn scene(body: &str, background: Rgb) -> Scene {
        let deck = Arc::new(SlideDeck {
            title: "Test".into(),
            kind: ContentKind::Song,
            sections: Vec::new(),
            slides: vec![Slide {
                body: body.into(),
                label: None,
                footer: None,
            }],
        });
        let mut theme = SlideTheme::song();
        theme.background = background;
        Scene::new(Presentation::new(deck), theme, OutputState::Show)
    }

    fn fade() -> TransitionSpec {
        TransitionSpec {
            mode: TransitionMode::Fade,
            duration_ms: 1000,
        }
    }

    #[test]
    fn different_slides_fade_out_then_in_over_interpolated_background() {
        let now = Instant::now();
        let mut playback = Playback::new(scene("Old", Rgb::new(0.0, 0.0, 0.0)));
        assert!(playback.retarget(scene("New", Rgb::new(1.0, 0.5, 0.0)), fade(), now));
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "Old");
        assert_eq!(playback.frame().body_opacity, 1.0);
        playback.advance(now + Duration::from_millis(250));
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "Old");
        assert_eq!(playback.frame().body_opacity, 0.5);
        assert_eq!(playback.frame().background, Rgb::new(0.25, 0.125, 0.0));
        playback.advance(now + Duration::from_millis(500));
        assert_eq!(playback.frame().body_opacity, 0.0);
        playback.advance(now + Duration::from_millis(750));
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "New");
        assert_eq!(playback.frame().body_opacity, 0.5);
        playback.advance(now + Duration::from_secs(1));
        assert!(!playback.is_active());
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "New");
        assert_eq!(playback.frame().body_opacity, 1.0);
        assert_eq!(playback.frame().background, Rgb::new(1.0, 0.5, 0.0));
    }
    #[test]
    fn same_scene_modes_interpolate_opacity_across_the_complete_duration() {
        let now = Instant::now();
        let original = scene("Amen", Rgb::new(1.0, 0.5, 0.0));
        let mut hidden = original.clone();
        hidden.state = OutputState::Black;
        let mut playback = Playback::new(original.clone());
        playback.retarget(hidden, fade(), now);
        playback.advance(now + Duration::from_millis(250));
        assert_eq!(playback.frame().body_opacity, 0.75);
        assert_eq!(playback.frame().background, Rgb::new(0.75, 0.375, 0.0));
        playback.advance(now + Duration::from_secs(1));
        playback.retarget(original.clone(), fade(), now + Duration::from_secs(1));
        playback.advance(now + Duration::from_millis(1250));
        assert_eq!(playback.frame().body_opacity, 0.25);
        playback.advance(now + Duration::from_secs(2));
        let mut clear = original.clone();
        clear.state = OutputState::Clear;
        playback.retarget(clear, fade(), now + Duration::from_secs(2));
        playback.advance(now + Duration::from_millis(2250));
        assert_eq!(playback.frame().body_opacity, 0.75);
        assert_eq!(playback.frame().background, original.theme.background);
    }

    #[test]
    fn an_incoming_body_fades_in_immediately_when_the_old_scene_has_no_visible_text() {
        let now = Instant::now();
        let mut hidden = scene("Old", Rgb::new(0.0, 0.0, 0.0));
        hidden.state = OutputState::Clear;
        let mut playback = Playback::new(hidden);
        playback.retarget(scene("New", Rgb::new(1.0, 0.0, 0.0)), fade(), now);
        playback.advance(now + Duration::from_millis(250));
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "New");
        assert_eq!(playback.frame().body_opacity, 0.25);
    }

    #[test]
    fn interruption_preserves_the_current_visible_body_opacity_and_background() {
        let now = Instant::now();
        let mut playback = Playback::new(scene("First", Rgb::new(0.0, 0.0, 0.0)));
        playback.retarget(scene("Second", Rgb::new(1.0, 0.0, 0.0)), fade(), now);
        playback.advance(now + Duration::from_millis(250));
        playback.retarget(
            scene("Third", Rgb::new(0.0, 1.0, 0.0)),
            fade(),
            now + Duration::from_millis(250),
        );
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "First");
        assert_eq!(playback.frame().body_opacity, 0.5);
        assert_eq!(playback.frame().background, Rgb::new(0.25, 0.0, 0.0));
        playback.advance(now + Duration::from_millis(1000));
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "Third");
        playback.retarget(
            scene("Fourth", Rgb::new(0.0, 0.0, 1.0)),
            fade(),
            now + Duration::from_millis(1000),
        );
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "Third");
        assert_eq!(playback.frame().body_opacity, 0.5);
        assert_eq!(playback.frame().background, Rgb::new(0.0625, 0.75, 0.0));
    }

    #[test]
    fn repeated_target_commands_do_not_restart_the_active_fade() {
        let now = Instant::now();
        let target = scene("Target", Rgb::new(1.0, 0.0, 0.0));
        let mut playback = Playback::new(scene("Source", Rgb::new(0.0, 0.0, 0.0)));
        playback.retarget(target.clone(), fade(), now);
        assert!(!playback.retarget(target.clone(), fade(), now + Duration::from_millis(250)));
        playback.advance(now + Duration::from_secs(1));
        assert!(!playback.is_active());
        assert!(!playback.retarget(target, fade(), now + Duration::from_secs(2)));
        assert!(!playback.is_active());
    }

    #[test]
    fn older_or_duplicate_frames_never_reverse_progress_or_rebased_time() {
        let now = Instant::now();
        let mut playback = Playback::new(scene("Source", Rgb::new(0.0, 0.0, 0.0)));
        playback.retarget(scene("Target", Rgb::new(1.0, 0.0, 0.0)), fade(), now);
        assert!(playback.advance(now + Duration::from_millis(750)));
        assert!(!playback.advance(now + Duration::from_millis(750)));
        assert!(!playback.advance(now + Duration::from_millis(250)));
        assert!(!playback.advance(now - Duration::from_secs(1)));
        assert_eq!(playback.frame().body_opacity, 0.5);
        assert_eq!(playback.frame().background, Rgb::new(0.75, 0.0, 0.0));
        playback.retarget(
            scene("Next", Rgb::new(0.0, 1.0, 0.0)),
            fade(),
            now + Duration::from_millis(250),
        );
        assert!(!playback.advance(now + Duration::from_millis(500)));
        assert_eq!(playback.frame().body_opacity, 0.5);
        assert_eq!(playback.frame().background, Rgb::new(0.75, 0.0, 0.0));
    }

    #[test]
    fn completion_releases_outgoing_decks_and_keeps_the_exact_target() {
        let now = Instant::now();
        let source = scene("Source", Rgb::new(0.0, 0.0, 0.0));
        let source_deck = Arc::downgrade(&source.presentation.as_ref().unwrap().share());
        let mut playback = Playback::new(source);
        let target = scene("Target", Rgb::new(1.0, 0.0, 0.0));
        let target_deck = target.presentation.as_ref().unwrap().share();
        playback.retarget(target, fade(), now);
        assert!(source_deck.upgrade().is_some());
        playback.advance(now + Duration::from_secs(3));
        assert!(source_deck.upgrade().is_none());
        assert!(!playback.is_active());
        assert_eq!(
            playback
                .target()
                .presentation
                .as_ref()
                .unwrap()
                .current()
                .body
                .as_ref(),
            "Target"
        );
        assert!(std::ptr::eq(
            playback.frame().slide.unwrap(),
            &target_deck.slides[0]
        ));
        assert!(!playback.advance(now + Duration::from_secs(4)));
    }

    #[test]
    fn cut_and_incompatible_layouts_switch_immediately() {
        let now = Instant::now();
        let mut playback = Playback::new(scene("Source", Rgb::new(0.0, 0.0, 0.0)));
        playback.retarget(
            scene("Cut", Rgb::new(1.0, 0.0, 0.0)),
            TransitionSpec::default(),
            now,
        );
        assert!(!playback.is_active());
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "Cut");
        let mut target = scene("Layout", Rgb::new(0.0, 1.0, 0.0));
        target.theme.layout.format = sdw_core::settings::CanvasFormat::Standard;
        playback.retarget(target, fade(), now);
        assert!(!playback.is_active());
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "Layout");
        let mut target = scene("Margin", Rgb::new(0.0, 0.0, 1.0));
        target.theme.layout.format = sdw_core::settings::CanvasFormat::Standard;
        target.theme.margin = 150.0;
        playback.retarget(target, fade(), now);
        assert!(!playback.is_active());
        assert_eq!(playback.frame().slide.unwrap().body.as_ref(), "Margin");
    }
}

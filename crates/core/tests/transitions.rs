use sdw_core::Settings;
use sdw_core::settings::SettingsError;
use sdw_core::transition::{TransitionMode, TransitionSpec};

#[test]
fn transition_preferences_keep_independent_modes_and_bounded_remembered_durations() {
    let mut settings = Settings::default();
    assert_eq!(
        settings.transitions.slides,
        TransitionSpec {
            mode: TransitionMode::Cut,
            duration_ms: 500
        }
    );
    settings.transitions.black.mode = TransitionMode::Fade;
    settings.transitions.clear.duration_ms = 2000;
    assert_eq!(settings.validate(), Ok(()));
    for duration_ms in [0, 99, 125, 2001, u16::MAX] {
        settings.transitions.slides.duration_ms = duration_ms;
        assert_eq!(settings.validate(), Err(SettingsError::InvalidTransitions));
    }
    settings.transitions.slides.duration_ms = 1;
    settings.transitions.black.duration_ms = 327;
    settings.transitions.clear.duration_ms = 4000;
    let clamped = settings.clamped();
    assert_eq!(clamped.transitions.slides.duration_ms, 100);
    assert_eq!(clamped.transitions.black.duration_ms, 350);
    assert_eq!(clamped.transitions.clear.duration_ms, 2000);
    assert_eq!(clamped.transitions.black.mode, TransitionMode::Fade);
    assert_eq!(clamped.validate(), Ok(()));
}

#[test]
fn aggregate_settings_validate_and_recover_label_conflicts() {
    let mut settings = Settings::default();
    settings.labels.rules[1].shortcut = Some('V');
    assert_eq!(settings.validate(), Err(SettingsError::ConflictingLabels));
    assert_eq!(settings.clamped().labels, Settings::default().labels);
}

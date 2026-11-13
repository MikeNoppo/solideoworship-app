use sdw_core::Settings;
use sdw_core::settings::{CanvasFormat, FontSpec, FontWeight, OutputLayout, ScreenMargins};

#[test]
fn defaults_preserve_projection_and_general_behavior() {
    let settings = Settings::default();

    assert!(!settings.general.open_output_on_startup);
    assert!(!settings.general.advance_after_go_live);
    assert!(settings.general.shadows);
    assert_eq!(settings.output.monitor, None);
    assert_eq!(settings.output.layout.format, CanvasFormat::Wide);
    assert_eq!(settings.output.layout.margins, ScreenMargins::default());
    assert_eq!(settings.output.layout.ratio(), 16.0 / 9.0);
    assert_eq!(settings.output.layout.height(), 1080.0);
    assert_eq!(settings.styles.song.theme.layout, OutputLayout::default());
    assert_eq!(
        settings.styles.scripture.theme.layout,
        OutputLayout::default()
    );
    assert_eq!(
        settings.styles.song.theme.body_font,
        FontSpec {
            family: "Inter".into(),
            weight: FontWeight::Normal
        }
    );
    assert_eq!(settings.styles.song.theme.footer_font, FontSpec::default());
    assert!(!settings.styles.song.theme.show_label);
    assert!(!settings.styles.song.theme.text_shadow);
    assert!(!settings.styles.song.theme.text_scrim);
    assert!(settings.styles.song.show_copyright);
    assert!(!settings.styles.song.copyright_first_slide_only);
    assert!(settings.styles.song.license.is_empty());
    assert!(settings.styles.scripture.show_verse_numbers);
    assert!(settings.styles.scripture.show_reference);
    assert!(settings.styles.scripture.show_translation);
    assert_eq!(
        settings.styles.scripture.reference_position,
        sdw_core::ReferencePosition::Bottom
    );
    assert_eq!(
        settings.styles.scripture.reference_format,
        sdw_core::ReferenceFormat::Parentheses
    );
    assert!(!settings.styles.scripture.one_verse_per_slide);
    assert_eq!(
        settings.output.logo_background,
        sdw_core::theme::Rgb::from_rgb8(20, 20, 22)
    );
    assert_eq!(
        settings.transitions.logo,
        sdw_core::transition::TransitionSpec::default()
    );
}

#[test]
fn output_content_area_combines_safe_margins_and_padding() {
    let layout = OutputLayout {
        format: CanvasFormat::Standard,
        margins: ScreenMargins {
            left: 20.0,
            right: 40.0,
            top: 60.0,
            bottom: 80.0,
        },
    };
    let area = layout.content_area(120.0).unwrap();

    assert_eq!(layout.height(), 1440.0);
    assert_eq!(
        (area.x, area.y, area.width, area.height),
        (140.0, 180.0, 1620.0, 1060.0)
    );
}

#[test]
fn validation_rejects_invalid_numeric_preferences() {
    use sdw_core::settings::SettingsError;

    assert_eq!(Settings::default().validate(), Ok(()));

    for value in [f32::NAN, f32::INFINITY, -1.0, 201.0] {
        let mut settings = Settings::default();
        settings.styles.song.theme.body_size = value;
        assert_eq!(settings.validate(), Err(SettingsError::InvalidValues));
    }

    let mut settings = Settings::default();
    settings.styles.scripture.max_chars = 0;
    assert_eq!(settings.validate(), Err(SettingsError::InvalidValues));
    settings.styles.scripture.max_chars = 220;
    settings.output.layout.margins.top = f32::NAN;
    assert_eq!(settings.validate(), Err(SettingsError::InvalidValues));
}

#[test]
fn validation_requires_positive_content_area_for_both_styles() {
    use sdw_core::settings::SettingsError;

    let mut settings = Settings::default();
    settings.styles.scripture.theme.margin = 400.0;
    settings.output.layout.margins.top = 140.0;
    settings.output.layout.margins.bottom = 140.0;

    assert_eq!(settings.validate(), Err(SettingsError::EmptyContentArea));
    assert_eq!(settings.output.layout.content_area(400.0), None);
    settings.output.layout.margins.bottom = 139.0;
    assert_eq!(settings.validate(), Ok(()));
}

#[test]
fn loaded_settings_clamp_values_and_share_the_canonical_layout() {
    let mut settings = Settings::default();
    settings.output.layout.format = CanvasFormat::Standard;
    settings.output.layout.margins.left = 70.0;
    settings.styles.song.theme.body_size = f32::NAN;
    settings.styles.scripture.theme.footer_font.family = " ".into();
    settings.styles.song.theme.layout.margins.left = 300.0;

    let settings = settings.clamped();

    assert_eq!(settings.styles.song.theme.body_size, 24.0);
    assert_eq!(
        settings.styles.scripture.theme.footer_font.family.as_ref(),
        "Inter"
    );
    assert_eq!(settings.styles.song.theme.layout, settings.output.layout);
    assert_eq!(
        settings.styles.scripture.theme.layout,
        settings.output.layout
    );
    assert_eq!(settings.validate(), Ok(()));

    let mut invalid = settings;
    invalid.output.layout.margins.top = f32::INFINITY;
    invalid.output.layout.margins.left = 1920.0;
    let recovered = invalid.clamped();
    assert_eq!(recovered.output.layout.format, CanvasFormat::Standard);
    assert_eq!(recovered.output.layout.margins, ScreenMargins::default());
    assert_eq!(recovered.validate(), Ok(()));
}

#[test]
fn validation_rejects_unusable_monitor_descriptors() {
    use sdw_core::settings::MonitorPreference;

    let mut settings = Settings::default();
    settings.output.monitor = Some(MonitorPreference {
        backend_name: "HDMI-1".into(),
        friendly_name: "Projector".into(),
        width: 0,
        height: 1080,
    });
    assert!(settings.validate().is_err());

    let monitor = settings.output.monitor.as_mut().unwrap();
    monitor.width = 1920;
    monitor.backend_name = " ".into();
    assert!(settings.validate().is_err());
    assert_eq!(settings.clamped().output.monitor, None);
}

#[test]
fn validation_rejects_out_of_range_logo_background_and_invalid_logo_transition() {
    let mut settings = Settings::default();
    settings.output.logo_background.r = 1.5;
    assert!(settings.validate().is_err());
    let clamped = settings.clone().clamped();
    assert_eq!(clamped.output.logo_background.r, 1.0);
    assert_eq!(clamped.validate(), Ok(()));

    let mut invalid_transition = Settings::default();
    invalid_transition.transitions.logo.duration_ms = 125;
    assert_eq!(
        invalid_transition.validate(),
        Err(sdw_core::settings::SettingsError::InvalidTransitions)
    );
    assert_eq!(
        invalid_transition.clamped().transitions.logo.duration_ms,
        150
    );
}

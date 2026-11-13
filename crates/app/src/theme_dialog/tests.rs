use sdw_core::slide::ContentKind;
use sdw_core::theme::{MAX_LINES, Rgb, TextAlign};
use sdw_core::{ScheduleItem, Song, Styles};

use super::{Action, Category, ColorSlot, FontSlot, MarginSide, Message, ThemeDialog};

fn dialog() -> ThemeDialog {
    let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
    let _ = dialog.update(Message::CategorySelected(Category::Song));
    dialog
}

#[test]
fn transition_demo_uses_sample_sections_for_a_single_slide_selected_song() {
    let current = ScheduleItem::Song(Song::new("Selected", "single slide", None));
    let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), Some(&current), &[]);
    let _ = dialog.update(Message::TransitionMode(
        super::TransitionTarget::Slides,
        sdw_core::transition::TransitionMode::Fade,
    ));
    let _ = dialog.update(Message::PreviewTransition(super::TransitionTarget::Slides));
    assert!(dialog.has_animation());
}

#[test]
fn transition_demo_finishes_and_stops_when_saving_resetting_or_closing() {
    use super::TransitionTarget;
    use sdw_core::transition::TransitionMode;
    use std::time::{Duration, Instant};
    for stop in [
        Message::Reset,
        Message::Cancel,
        Message::Apply,
        Message::Save,
        Message::CategorySelected(Category::Song),
    ] {
        let mut dialog = dialog();
        let _ = dialog.update(Message::CategorySelected(Category::Transitions));
        let _ = dialog.update(Message::TransitionMode(
            TransitionTarget::Slides,
            TransitionMode::Fade,
        ));
        assert_eq!(
            dialog.update(Message::PreviewTransition(TransitionTarget::Slides)),
            Action::None
        );
        assert!(dialog.has_animation());
        let _ = dialog.update(stop);
        assert!(!dialog.has_animation());
    }
    let mut dialog = dialog();
    let _ = dialog.update(Message::TransitionMode(
        TransitionTarget::Slides,
        TransitionMode::Fade,
    ));
    let _ = dialog.update(Message::PreviewTransition(TransitionTarget::Slides));
    assert!(dialog.advance_animation(Instant::now() + Duration::from_secs(3)));
    assert!(!dialog.has_animation());
    assert!(!dialog.advance_animation(Instant::now() + Duration::from_secs(4)));
}

#[test]
fn transition_modes_preserve_independent_durations_and_reject_invalid_raw_text() {
    use super::TransitionTarget;
    use sdw_core::transition::TransitionMode;
    let mut dialog = dialog();
    let _ = dialog.update(Message::TransitionMode(
        TransitionTarget::Slides,
        TransitionMode::Fade,
    ));
    let _ = dialog.update(Message::TransitionDuration(
        TransitionTarget::Slides,
        "850".into(),
    ));
    let _ = dialog.update(Message::TransitionMode(
        TransitionTarget::Slides,
        TransitionMode::Cut,
    ));
    let _ = dialog.update(Message::TransitionDuration(
        TransitionTarget::Black,
        "550x".into(),
    ));
    let _ = dialog.update(Message::CategorySelected(Category::General));
    assert_eq!(dialog.update(Message::Apply), Action::None);
    let _ = dialog.update(Message::TransitionDuration(
        TransitionTarget::Black,
        "650".into(),
    ));
    let Action::SaveSettings(saved, _) = dialog.update(Message::Apply) else {
        panic!("expected independent transition preferences");
    };
    assert_eq!(saved.transitions.slides.mode, TransitionMode::Cut);
    assert_eq!(saved.transitions.slides.duration_ms, 850);
    assert_eq!(saved.transitions.black.duration_ms, 650);
    assert_eq!(saved.transitions.clear.duration_ms, 500);
}

#[test]
fn label_view_does_not_copy_name_and_alias_buffers() {
    let mut dialog = dialog();
    let name = "n".repeat(61);
    let aliases = std::array::from_fn::<_, 8, _>(|_| "a".repeat(61)).join(", ");
    for index in 0..8 {
        let _ = dialog.update(Message::LabelChanged(
            index,
            super::LabelField::Name,
            name.clone(),
        ));
        let _ = dialog.update(Message::LabelChanged(
            index,
            super::LabelField::Aliases,
            aliases.clone(),
        ));
    }
    let _ = dialog.update(Message::CategorySelected(Category::Labels));
    let copies = crate::allocation_probe::measure(|| {
        std::hint::black_box(dialog.view(1480.0));
    });
    eprintln!("label name/alias allocations per redraw: {copies}");
    assert_eq!(copies, 0);
}

#[test]
fn label_table_accepts_at_most_thirty_two_rules() {
    let mut dialog = dialog();
    for index in 8..32 {
        let _ = dialog.update(Message::AddLabel);
        let name = format!("Custom{}", char::from(b'A' + (index - 8) as u8));
        let _ = dialog.update(Message::LabelChanged(index, super::LabelField::Name, name));
    }
    let _ = dialog.update(Message::AddLabel);
    let Action::SaveSettings(saved, _) = dialog.update(Message::Apply) else {
        panic!("expected bounded label table");
    };
    assert_eq!(saved.labels.rules.len(), 32);
}

#[test]
fn label_conflicts_and_malformed_aliases_or_shortcuts_block_saving() {
    use super::LabelField;
    for (field, value) in [
        (LabelField::Name, "Chorus"),
        (LabelField::Aliases, "new name, Chorus"),
        (LabelField::Aliases, "valid,"),
        (LabelField::Shortcut, "C"),
        (LabelField::Shortcut, "vv"),
        (LabelField::Shortcut, "v"),
        (LabelField::Shortcut, "1"),
    ] {
        let mut dialog = dialog();
        let _ = dialog.update(Message::LabelChanged(0, field, value.into()));
        assert_eq!(dialog.update(Message::Apply), Action::None);
    }
}

#[test]
fn label_add_delete_and_category_reset_preserve_unrelated_settings() {
    use super::LabelField;
    let mut dialog = dialog();
    let _ = dialog.update(Message::OpenAtStartup(true));
    let _ = dialog.update(Message::DeleteLabel(0));
    let _ = dialog.update(Message::AddLabel);
    let _ = dialog.update(Message::LabelChanged(7, LabelField::Name, "Solo".into()));
    let _ = dialog.update(Message::LabelChanged(
        7,
        LabelField::Aliases,
        "Instrumental, Soloist".into(),
    ));
    let _ = dialog.update(Message::LabelChanged(7, LabelField::Shortcut, "S".into()));
    let Action::SaveSettings(saved, false) = dialog.update(Message::Apply) else {
        panic!("expected added label to save");
    };
    assert_eq!(saved.labels.rules.len(), 8);
    assert_eq!(&*saved.labels.rules[7].name, "Solo");
    assert_eq!(saved.labels.rules[7].shortcut, Some('S'));
    let _ = dialog.update(Message::DeleteLabel(7));
    dialog.saved(&saved);
    let _ = dialog.update(Message::CategorySelected(Category::Labels));
    let _ = dialog.update(Message::Reset);
    let Action::SaveSettings(saved, _) = dialog.update(Message::Apply) else {
        panic!("expected reset label settings");
    };
    assert_eq!(saved.labels, sdw_core::labels::LabelSettings::default());
    assert!(saved.general.open_output_on_startup);
}

#[test]
fn transition_reset_restores_only_transitions_and_clears_invalid_duration() {
    use super::TransitionTarget;
    let mut dialog = dialog();
    let _ = dialog.update(Message::LicenseChanged("CCLI 123".into()));
    let _ = dialog.update(Message::TransitionDuration(
        TransitionTarget::Slides,
        "invalid".into(),
    ));
    let _ = dialog.update(Message::CategorySelected(Category::Transitions));
    let _ = dialog.update(Message::Reset);
    let Action::SaveSettings(saved, _) = dialog.update(Message::Apply) else {
        panic!("expected reset transition settings");
    };
    assert_eq!(
        saved.transitions,
        sdw_core::transition::TransitionSettings::default()
    );
    assert_eq!(&*saved.styles.song.license, "CCLI 123");
}

#[test]
fn invalid_label_draft_blocks_apply_from_another_category() {
    let mut dialog = dialog();
    let _ = dialog.update(Message::LabelChanged(
        0,
        super::LabelField::Color,
        "#12".into(),
    ));
    let _ = dialog.update(Message::CategorySelected(Category::General));
    assert_eq!(dialog.update(Message::Apply), Action::None);
    let _ = dialog.update(Message::LabelChanged(
        0,
        super::LabelField::Color,
        "#123456".into(),
    ));
    let Action::SaveSettings(settings, false) = dialog.update(Message::Apply) else {
        panic!("expected corrected label color to save");
    };
    assert_eq!(settings.labels.rules[0].color, Rgb::from_rgb8(18, 52, 86));
}

#[test]
fn apply_saves_the_draft_and_reopens_editing_after_success() {
    let settings = sdw_core::Settings::default();
    let mut dialog = ThemeDialog::settings(&settings, None, &[]);
    let _ = dialog.update(Message::BodySize(110.0));

    let Action::SaveSettings(saved, false) = dialog.update(Message::Apply) else {
        panic!("expected an apply action");
    };
    assert_eq!(saved.styles.song.theme.body_size, 110.0);
    assert_eq!(dialog.update(Message::Cancel), Action::None);
    let _ = dialog.update(Message::BodySize(150.0));
    dialog.saved(&saved);

    let Action::SaveSettings(saved, true) = dialog.update(Message::Save) else {
        panic!("expected an ok action");
    };
    assert_eq!(saved.styles.song.theme.body_size, 110.0);
}

#[test]
fn stale_monitor_selection_keeps_the_remembered_monitor() {
    let mut settings = sdw_core::Settings::default();
    settings.output.monitor = Some(sdw_core::settings::MonitorPreference {
        backend_name: "HDMI-1".into(),
        friendly_name: "Projector".into(),
        width: 1920,
        height: 1080,
    });
    let mut dialog = ThemeDialog::settings(&settings, None, &[]);
    let _ = dialog.update(Message::MonitorSelected(Some(99)));
    let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
        panic!("expected remembered monitor preferences");
    };
    assert_eq!(saved.output.monitor, settings.output.monitor);
}

#[test]
fn song_and_scripture_options_save_without_changing_the_original_settings() {
    let original = sdw_core::Settings::default();
    let mut dialog = ThemeDialog::settings(&original, None, &[]);
    let _ = dialog.update(Message::CategorySelected(Category::Song));
    let _ = dialog.update(Message::ShowHeading(true));
    let _ = dialog.update(Message::ShowCopyright(false));
    let _ = dialog.update(Message::LicenseChanged("CCLI 12345".into()));
    let _ = dialog.update(Message::CategorySelected(Category::Scripture));
    let _ = dialog.update(Message::VerseNumbers(false));
    let _ = dialog.update(Message::Reference(false));
    let _ = dialog.update(Message::Translation(false));
    let _ = dialog.update(Message::OneVerse(true));
    let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
        panic!("expected content preferences");
    };
    assert!(saved.styles.song.theme.show_label);
    assert!(!saved.styles.song.show_copyright);
    assert_eq!(&*saved.styles.song.license, "CCLI 12345");
    assert!(!saved.styles.scripture.show_verse_numbers);
    assert!(!saved.styles.scripture.show_reference);
    assert!(!saved.styles.scripture.show_translation);
    assert!(saved.styles.scripture.one_verse_per_slide);
    assert_eq!(original, sdw_core::Settings::default());
}

#[test]
fn body_and_footer_fonts_are_independent_and_search_does_not_select() {
    let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
    let _ = dialog.update(Message::FontSelected(FontSlot::Body, "JetBrains Mono"));
    let _ = dialog.update(Message::FontWeight(
        FontSlot::Body,
        sdw_core::settings::FontWeight::Bold,
    ));
    let _ = dialog.update(Message::FontWeight(
        FontSlot::Footer,
        sdw_core::settings::FontWeight::Medium,
    ));
    let _ = dialog.update(Message::FontSearch(FontSlot::Footer, "Mono".into()));
    let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
        panic!("expected font preferences");
    };
    assert_eq!(&*saved.styles.song.theme.body_font.family, "JetBrains Mono");
    assert_eq!(
        saved.styles.song.theme.body_font.weight,
        sdw_core::settings::FontWeight::Bold
    );
    assert_eq!(&*saved.styles.song.theme.footer_font.family, "Inter");
    assert_eq!(
        saved.styles.song.theme.footer_font.weight,
        sdw_core::settings::FontWeight::Medium
    );
    assert_eq!(&*saved.styles.scripture.theme.body_font.family, "Inter");
}

#[test]
fn invalid_margin_text_and_empty_content_area_block_saving() {
    let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
    let _ = dialog.update(Message::SafeMargin(MarginSide::Left, "abc".into()));
    assert_eq!(dialog.update(Message::Save), Action::None);
    let _ = dialog.update(Message::SafeMargin(MarginSide::Left, "1800".into()));
    assert_eq!(dialog.update(Message::Save), Action::None);
    let _ = dialog.update(Message::SafeMargin(MarginSide::Left, "80".into()));
    let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
        panic!("expected valid margins to save");
    };
    assert_eq!(saved.output.layout.margins.left, 80.0);
    assert_eq!(saved.styles.song.theme.layout, saved.output.layout);
    assert_eq!(saved.styles.scripture.theme.layout, saved.output.layout);
}

#[test]
fn category_reset_only_restores_the_selected_preferences() {
    let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
    let _ = dialog.update(Message::OpenAtStartup(true));
    let _ = dialog.update(Message::AdvanceAfterGoLive(true));
    let _ = dialog.update(Message::CategorySelected(Category::Output));
    let _ = dialog.update(Message::CanvasFormat(
        sdw_core::settings::CanvasFormat::Standard,
    ));
    let _ = dialog.update(Message::SafeMargin(MarginSide::Left, "80".into()));
    let _ = dialog.update(Message::Reset);
    let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
        panic!("expected settings after resetting output");
    };
    assert!(saved.general.open_output_on_startup);
    assert!(saved.general.advance_after_go_live);
    assert_eq!(saved.output, sdw_core::settings::OutputSettings::default());
}

#[test]
fn refreshing_displays_preserves_the_unsaved_draft() {
    let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
    let _ = dialog.update(Message::BodySize(110.0));
    assert_eq!(
        dialog.update(Message::RefreshDisplays),
        Action::RefreshDisplays
    );
    assert_eq!(dialog.update(Message::RefreshDisplays), Action::None);
    dialog.set_displays(&[]);
    assert_eq!(
        dialog.update(Message::RefreshDisplays),
        Action::RefreshDisplays
    );
    dialog.fail("unavailable");
    let Action::SaveSettings(saved, _) = dialog.update(Message::Save) else {
        panic!("expected settings after refreshing monitors");
    };
    assert_eq!(saved.styles.song.theme.body_size, 110.0);
}

#[test]
fn invalid_hex_blocks_saving_until_corrected() {
    let mut dialog = ThemeDialog::settings(&sdw_core::Settings::default(), None, &[]);
    let _ = dialog.update(Message::HexChanged(ColorSlot::Body, "#12".into()));

    assert_eq!(dialog.update(Message::Save), Action::None);
    let _ = dialog.update(Message::HexChanged(ColorSlot::Body, "#ff0000".into()));
    assert!(matches!(
        dialog.update(Message::Save),
        Action::SaveSettings(..)
    ));
}

#[test]
fn without_selected_item_starts_on_song_tab() {
    assert_eq!(dialog().kind, ContentKind::Song);
}

#[test]
fn selected_item_becomes_sample_and_picks_tab() {
    let item = ScheduleItem::Song(Song::new("Selected", "one", None));
    let dialog = ThemeDialog::settings(&sdw_core::Settings::default(), Some(&item), &[]);

    assert_eq!(dialog.kind, ContentKind::Song);
    assert_eq!(&*dialog.deck.title, "Selected");
}

#[test]
fn scripture_sample_available_without_selected_item() {
    let mut dialog = dialog();

    let _ = dialog.update(Message::KindSelected(ContentKind::Scripture));

    assert!(dialog.deck.title.contains("Mazmur 23"));
}

#[test]
fn line_limit_recompiles_preview() {
    let mut dialog = dialog();
    let before = dialog.deck.len();

    let _ = dialog.update(Message::MaxLines(1));

    assert!(dialog.deck.len() > before);
}

#[test]
fn invalid_hex_does_not_change_color() {
    let mut dialog = dialog();
    let original = dialog.draft.styles.song.theme.background;

    let _ = dialog.update(Message::HexChanged(ColorSlot::Background, "#12".into()));

    assert_eq!(dialog.draft.styles.song.theme.background, original);
    assert_eq!(dialog.hex[0][0], "#12");
}

#[test]
fn valid_hex_applies_immediately() {
    let mut dialog = dialog();

    let _ = dialog.update(Message::HexChanged(ColorSlot::Body, "#ff0000".into()));

    assert_eq!(
        dialog.draft.styles.song.theme.body,
        Rgb::from_rgb8(255, 0, 0)
    );
}

#[test]
fn preset_color_updates_hex_field() {
    let mut dialog = dialog();

    let _ = dialog.update(Message::ColorPicked(
        ColorSlot::Footer,
        Rgb::from_rgb8(0, 0, 0),
    ));

    assert_eq!(dialog.hex[0][2], "#000000");
}

#[test]
fn changes_only_affect_active_kind() {
    let mut dialog = dialog();

    let _ = dialog.update(Message::Align(TextAlign::Left));

    assert_eq!(dialog.draft.styles.song.theme.align, TextAlign::Left);
    assert_eq!(dialog.draft.styles.scripture.theme.align, TextAlign::Center);
}

#[test]
fn reset_defaults_only_affects_active_kind() {
    let mut dialog = dialog();
    let _ = dialog.update(Message::BodySize(50.0));
    let _ = dialog.update(Message::KindSelected(ContentKind::Scripture));
    let _ = dialog.update(Message::BodySize(40.0));

    let _ = dialog.update(Message::Reset);

    assert_eq!(dialog.draft.styles.song.theme.body_size, 50.0);
    assert_eq!(
        dialog.draft.styles.scripture.theme.body_size,
        Styles::default().scripture.theme.body_size
    );
}

#[test]
fn save_sends_clamped_settings_once() {
    let mut dialog = dialog();
    let _ = dialog.update(Message::MaxLines(500));

    let Action::SaveSettings(settings, true) = dialog.update(Message::Save) else {
        panic!("expected a save action");
    };

    assert_eq!(settings.styles.song.max_lines, *MAX_LINES.end());
    assert_eq!(dialog.update(Message::Save), Action::None);
}

#[test]
fn failed_save_re_enables_button() {
    let mut dialog = dialog();
    let _ = dialog.update(Message::Save);

    dialog.fail("broken.");

    assert!(dialog.error.as_deref().unwrap().contains("broken"));
    assert!(!dialog.error.as_deref().unwrap().contains("broken.."));
    assert!(matches!(
        dialog.update(Message::Save),
        Action::SaveSettings(..)
    ));
}

#[test]
fn shadows_choice_is_saved_with_settings() {
    let mut dialog = dialog();
    let _ = dialog.update(Message::Shadows(false));

    let Action::SaveSettings(settings, true) = dialog.update(Message::Save) else {
        panic!("expected a settings save action");
    };
    assert!(!settings.general.shadows);
}

#[test]
fn steppers_stay_within_their_ranges() {
    let mut dialog = dialog();
    let _ = dialog.update(Message::MaxLines(0));
    let _ = dialog.update(Message::MaxChars(5000));

    assert_eq!(dialog.draft.styles.song.max_lines, *MAX_LINES.start());
    assert_eq!(
        dialog.draft.styles.scripture.max_chars,
        *sdw_core::theme::MAX_CHARS.end()
    );
}

#[test]
fn cancel_waits_for_a_running_save() {
    let mut dialog = dialog();
    let _ = dialog.update(Message::Save);

    assert_eq!(dialog.update(Message::Cancel), Action::None);
}

#[test]
fn logo_background_and_logo_transitions_validate_save_and_reset() {
    use super::TransitionTarget;
    use sdw_core::OutputState;
    use sdw_core::transition::TransitionMode;

    let mut dialog = dialog();
    let _ = dialog.update(Message::CategorySelected(Category::Output));
    let original = dialog.draft.output.logo_background;

    let _ = dialog.update(Message::LogoHexChanged("#12".into()));
    assert_eq!(dialog.draft.output.logo_background, original);
    assert_eq!(dialog.update(Message::Save), Action::None);
    assert!(dialog.error.is_some());

    let picked = Rgb::from_rgb8(24, 36, 48);
    let _ = dialog.update(Message::LogoColorPicked(picked));
    assert_eq!(dialog.logo_hex, "#182430");
    assert_eq!(dialog.draft.output.logo_background, picked);

    let _ = dialog.update(Message::CategorySelected(Category::Transitions));
    let _ = dialog.update(Message::TransitionMode(
        TransitionTarget::Logo,
        TransitionMode::Fade,
    ));
    let _ = dialog.update(Message::TransitionDuration(
        TransitionTarget::Logo,
        "650".into(),
    ));
    let _ = dialog.update(Message::PreviewTransition(TransitionTarget::Logo));
    assert_eq!(
        dialog.demo.as_ref().unwrap().target().state,
        OutputState::Logo
    );

    let _ = dialog.update(Message::CategorySelected(Category::Output));
    let _ = dialog.update(Message::Reset);
    assert_eq!(dialog.draft.output.logo_background, original);
    assert_eq!(dialog.logo_hex, original.to_hex());
    assert_eq!(dialog.draft.transitions.logo.mode, TransitionMode::Fade);
    assert_eq!(dialog.draft.transitions.logo.duration_ms, 650);

    let _ = dialog.update(Message::LogoHexChanged("#102030".into()));
    let Action::SaveSettings(settings, true) = dialog.update(Message::Save) else {
        panic!("expected a save action");
    };
    assert_eq!(settings.output.logo_background, Rgb::from_rgb8(16, 32, 48));
    assert_eq!(settings.transitions.logo.mode, TransitionMode::Fade);
    assert_eq!(settings.transitions.logo.duration_ms, 650);
}

#[test]
fn phase3_song_copyright_scripture_reference_and_readability_save_and_reset() {
    use sdw_core::theme::{ReferenceFormat, ReferencePosition};

    let mut dialog = dialog();
    let _ = dialog.update(Message::CopyrightFirstOnly(true));
    let _ = dialog.update(Message::TextShadow(true));
    let _ = dialog.update(Message::TextScrim(true));
    assert!(dialog.deck.slides.len() >= 2);
    assert!(dialog.deck.slides[0].footer.is_some());
    assert!(dialog.deck.slides[1].footer.is_none());

    let _ = dialog.update(Message::CategorySelected(Category::Scripture));
    let _ = dialog.update(Message::ReferencePosition(ReferencePosition::Top));
    let _ = dialog.update(Message::ReferenceFormat(ReferenceFormat::MiddleDot));
    let _ = dialog.update(Message::TextShadow(true));
    let first_slide = &dialog.deck.slides[0];
    assert!(
        first_slide
            .header
            .as_deref()
            .is_some_and(|h| h.contains(" · "))
    );
    assert!(first_slide.footer.is_none());
    assert!(!dialog.draft.styles.scripture.theme.text_scrim);

    let _ = dialog.update(Message::CategorySelected(Category::Song));
    let _ = dialog.update(Message::Reset);
    assert!(!dialog.draft.styles.song.copyright_first_slide_only);
    assert!(!dialog.draft.styles.song.theme.text_shadow);
    assert!(!dialog.draft.styles.song.theme.text_scrim);
    assert!(dialog.draft.styles.scripture.theme.text_shadow);
    assert_eq!(
        dialog.draft.styles.scripture.reference_position,
        ReferencePosition::Top
    );
    assert_eq!(
        dialog.draft.styles.scripture.reference_format,
        ReferenceFormat::MiddleDot
    );

    let Action::SaveSettings(settings, true) = dialog.update(Message::Save) else {
        panic!("expected a save action");
    };
    assert!(!settings.styles.song.copyright_first_slide_only);
    assert!(settings.styles.scripture.theme.text_shadow);
    assert_eq!(
        settings.styles.scripture.reference_position,
        ReferencePosition::Top
    );
    assert_eq!(
        settings.styles.scripture.reference_format,
        ReferenceFormat::MiddleDot
    );
}

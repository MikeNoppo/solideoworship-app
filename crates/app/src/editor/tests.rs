use std::sync::Arc;

use iced::widget::text_editor::{Action as EditorAction, Edit};
use sdw_core::format::{
    BoxFill, BulletStyle, TextOutline, TextShadowOverride, TextSize, VerticalAlign,
};
use sdw_core::theme::{FontWeight, Rgb, SlideTheme, TextAlign};
use sdw_core::{SlideBox, Song};

use super::canvas::Target;
use super::format_panel::{
    BorderMode, BoxShadowMode, CanvasAlign, ColorSlot, DimField, FillMode, FormatMessage,
    FormatTab, OrderAction, PaddingSide, TextShadowMode, TextSizeMode, TextSubTab,
};
use super::reflow::Section;
use super::{Action, Editor, Message, NoticeKind};

fn editor(lyrics: &str) -> Editor {
    Editor::with_style(
        None,
        &Song::new("Test", lyrics, None),
        &sdw_core::Styles::default().song,
    )
}

fn paste_inline(editor: &mut Editor, text: &str) {
    let _ = editor.update(Message::EditStarted(Target::Body));
    let _ = editor.update(Message::InlineBodyEdited(EditorAction::Edit(Edit::Paste(
        Arc::new(text.to_owned()),
    ))));
}

#[test]
fn slides_are_compiled_on_open() {
    let editor = editor("[Verse 1]\none\n\n[Reff]\ntwo");
    assert_eq!(editor.slides.len(), 2);
    assert_eq!(editor.deck.len(), 2);
}

#[test]
fn inline_canvas_editing_updates_slide_and_lyrics() {
    let mut editor = editor("");
    paste_inline(&mut editor, "asdasda");

    assert_eq!(&*editor.slides[0].body, "asdasda");
    assert_eq!(editor.deck.slides[0].body.as_ref(), "asdasda");
    assert!(editor.song().lyrics().contains("asdasda"));
}

#[test]
fn resizing_box_updates_deck_and_undo_redo_restores() {
    let mut editor = editor("one");
    let custom = SlideBox::new(0.1, 0.1, 0.8, 0.4);

    let _ = editor.update(Message::BoxChanged(Target::Body, custom));
    let _ = editor.update(Message::BoxCommitted);
    assert_eq!(editor.deck.slides[0].body_box, Some(custom));

    let _ = editor.update(Message::Undo);
    assert_eq!(editor.deck.slides[0].body_box, None);

    let _ = editor.update(Message::Redo);
    assert_eq!(editor.deck.slides[0].body_box, Some(custom));
}

#[test]
fn add_duplicate_move_and_delete_slide() {
    let mut editor = editor("one");
    let _ = editor.update(Message::AddSlide);
    assert_eq!(editor.slides.len(), 2);
    assert_eq!(editor.selected, 1);

    paste_inline(&mut editor, "two");
    let _ = editor.update(Message::MoveSlideUp);
    assert_eq!(editor.selected, 0);
    assert_eq!(&*editor.slides[0].body, "two");

    let _ = editor.update(Message::DuplicateSlide);
    assert_eq!(editor.slides.len(), 3);
    assert_eq!(&*editor.slides[1].body, "two");

    let _ = editor.update(Message::DeleteSlide);
    assert_eq!(editor.slides.len(), 2);
}

#[test]
fn reflow_recompiles_lyrics_into_slides() {
    let mut editor = editor("initial");
    let _ = editor.update(Message::ToggleReflow);
    assert!(editor.reflow.is_some());

    let _ = editor.update(Message::ReflowEdited(EditorAction::SelectAll));
    let _ = editor.update(Message::ReflowEdited(EditorAction::Edit(Edit::Paste(
        Arc::new("[Verse 1]\na\n\n[Chorus]\nb".to_owned()),
    ))));
    let _ = editor.update(Message::ReflowInsert(Section::Bridge));
    let _ = editor.update(Message::ReflowEdited(EditorAction::Edit(Edit::Paste(
        Arc::new("c".to_owned()),
    ))));
    let _ = editor.update(Message::ApplyReflow);

    assert!(editor.reflow.is_none());
    assert_eq!(editor.slides.len(), 3);
    assert_eq!(editor.slides[1].label.as_deref(), Some("Chorus"));
    assert_eq!(&*editor.slides[2].body, "c");
}

#[test]
fn apply_keeps_open_and_save_closes() {
    let mut editor = editor("one");
    let Action::Save { close, .. } = editor.update(Message::Apply) else {
        panic!("expected save action");
    };
    assert!(!close);

    let song = editor.song();
    let id = sdw_library::Library::open_in_memory()
        .unwrap()
        .insert_song(&song)
        .unwrap();
    editor.mark_applied(id, &song);

    let Action::Save { origin, close, .. } = editor.update(Message::Save) else {
        panic!("expected save action");
    };
    assert_eq!(origin, Some(id));
    assert!(close);
}

#[test]
fn save_without_title_is_rejected() {
    let mut editor = editor("one");
    let _ = editor.update(Message::TitleChanged("   ".into()));

    assert_eq!(editor.update(Message::Save), Action::None);
    assert!(editor.error.is_some());
}

#[test]
fn dismiss_exits_inline_edit_before_closing() {
    let mut editor = editor("");
    let _ = editor.update(Message::EditStarted(Target::Body));
    assert!(editor.editing.is_some());

    assert_eq!(editor.update(Message::Dismiss), Action::None);
    assert!(editor.editing.is_none());

    assert_eq!(editor.update(Message::Dismiss), Action::Close);
}

#[test]
fn double_click_without_edit_creates_no_phantom_undo_step() {
    let mut editor = editor("[Verse 1]\none");
    let _ = editor.update(Message::EditStarted(Target::Body));
    let _ = editor.update(Message::FinishInlineEdit);
    assert!(!editor.history.can_undo());

    let _ = editor.update(Message::EditStarted(Target::Footer));
    let _ = editor.update(Message::BoxSelected(None));
    assert!(!editor.history.can_undo());
}

#[test]
fn text_typing_coalesces_into_one_undo_step() {
    let mut editor = editor("one");
    let _ = editor.update(Message::TitleChanged("Test A".into()));
    let _ = editor.update(Message::TitleChanged("Test AB".into()));
    let _ = editor.update(Message::TitleChanged("Test ABC".into()));

    assert!(editor.history.can_undo());
    let _ = editor.update(Message::Undo);
    assert_eq!(editor.title, "Test");
    assert!(!editor.history.can_undo());
}

#[test]
fn reset_box_can_be_undone_and_clears_customized_when_matching_auto() {
    let mut editor = editor("[Verse 1]\none");
    let custom = SlideBox::new(0.1, 0.1, 0.8, 0.5);
    let _ = editor.update(Message::BoxChanged(Target::Body, custom));
    let _ = editor.update(Message::BoxCommitted);
    assert!(editor.customized);
    assert!(editor.has_changes());

    let _ = editor.update(Message::ResetBox);
    assert!(!editor.customized);
    assert!(!editor.has_changes());
    assert_eq!(editor.update(Message::Dismiss), Action::Close);

    let _ = editor.update(Message::Undo);
    assert_eq!(editor.deck.slides[0].body_box, Some(custom));
}

#[test]
fn apply_box_to_all_move_down_label_zoom_and_footer_visibility() {
    let mut style = sdw_core::Styles::default().song;
    style.copyright_first_slide_only = true;
    let mut editor = Editor::with_style(
        None,
        &Song::new("Test", "[Verse 1]\none\n\n[Chorus]\ntwo", None),
        &style,
    );
    assert!(editor.footer_visible_on_selected());

    let custom = SlideBox::new(0.2, 0.2, 0.6, 0.3);
    let _ = editor.update(Message::BoxChanged(Target::Body, custom));
    let _ = editor.update(Message::BoxCommitted);
    let _ = editor.update(Message::ApplyBoxToAll);
    assert_eq!(editor.deck.slides[1].body_box, Some(custom));

    let _ = editor.update(Message::MoveSlideDown);
    assert_eq!(editor.selected, 1);
    assert!(!editor.footer_visible_on_selected());

    let _ = editor.update(Message::SlideLabelChanged("Bridge".into()));
    assert_eq!(editor.slides[1].label.as_deref(), Some("Bridge"));

    let _ = editor.update(Message::ZoomChanged(1.2));
    assert!(!editor.zoom_fit);
    assert_eq!(&*editor.zoom_label, "120%");
    let _ = editor.update(Message::ToggleZoomFit);
    assert!(editor.zoom_fit);
}

fn send_format(editor: &mut Editor, msg: FormatMessage) {
    let _ = editor.update(Message::Format(msg));
}

#[test]
fn apply_box_to_all_sets_notice_detects_match_and_mark_applied_sets_saved() {
    let mut editor = editor("[Verse 1]\none\n\n[Chorus]\ntwo");
    let custom = SlideBox::new(0.15, 0.15, 0.7, 0.4);
    let _ = editor.update(Message::BoxChanged(Target::Body, custom));
    let _ = editor.update(Message::BoxCommitted);

    let _ = editor.update(Message::ApplyBoxToAll);
    let first = editor
        .notice
        .as_ref()
        .expect("expected notice after apply all");
    assert_eq!(first.kind, NoticeKind::BoxApplied);
    assert!(!first.text.is_empty());

    let _ = editor.update(Message::ApplyBoxToAll);
    let second = editor
        .notice
        .as_ref()
        .expect("expected already matched notice");
    assert_eq!(second.kind, NoticeKind::BoxAlreadyMatched);
    assert!(!second.text.is_empty());

    let _ = editor.update(Message::BoxChanged(
        Target::Body,
        SlideBox::new(0.2, 0.2, 0.6, 0.4),
    ));
    assert!(editor.notice.is_none());

    let song = editor.song();
    let id = sdw_library::Library::open_in_memory()
        .unwrap()
        .insert_song(&song)
        .unwrap();
    editor.mark_applied(id, &song);
    let saved = editor.notice.as_ref().expect("expected saved notice");
    assert_eq!(saved.kind, NoticeKind::Saved);
    assert!(!saved.text.is_empty());
}

#[test]
fn style_tab_messages_update_format_undo_redo_and_song_output() {
    let mut editor = editor("one");
    let _ = editor.update(Message::ToggleFormat);
    assert!(editor.format_open);

    send_format(&mut editor, FormatMessage::TargetSelected(Target::Body));
    send_format(&mut editor, FormatMessage::FillMode(FillMode::Solid));
    send_format(&mut editor, FormatMessage::FillOpacity(50.0));
    send_format(
        &mut editor,
        FormatMessage::ColorPresetPicked(ColorSlot::Fill, Rgb::new(0.2, 0.4, 0.6)),
    );
    send_format(
        &mut editor,
        FormatMessage::ColorHexChanged(ColorSlot::Fill, "#112233".into()),
    );
    send_format(&mut editor, FormatMessage::CommitInput);

    match editor.format.body.fill {
        Some(BoxFill::Solid { color, opacity }) => {
            assert_eq!(color.to_hex(), "#112233");
            assert!((opacity - 0.5).abs() < 0.01);
        }
        other => panic!("unexpected fill: {other:?}"),
    }

    send_format(&mut editor, FormatMessage::BorderMode(BorderMode::Solid));
    send_format(&mut editor, FormatMessage::BorderWidth("4".into()));
    send_format(&mut editor, FormatMessage::BorderRadius("12".into()));
    send_format(
        &mut editor,
        FormatMessage::ColorHexChanged(ColorSlot::Border, "#abcdef".into()),
    );
    send_format(&mut editor, FormatMessage::CommitInput);
    let border = editor.format.body.border.expect("expected solid border");
    assert!((border.width - 4.0).abs() < 0.01);
    assert!((border.radius - 12.0).abs() < 0.01);
    assert_eq!(border.color.to_hex(), "#abcdef");

    send_format(
        &mut editor,
        FormatMessage::BoxShadowMode(BoxShadowMode::Drop),
    );
    send_format(&mut editor, FormatMessage::BoxShadowOpacity(60.0));
    send_format(&mut editor, FormatMessage::BoxShadowBlur("10".into()));
    send_format(&mut editor, FormatMessage::BoxShadowOffsetX("3".into()));
    send_format(&mut editor, FormatMessage::BoxShadowOffsetY("-5".into()));
    send_format(&mut editor, FormatMessage::CommitInput);
    let shadow = editor.format.body.shadow.expect("expected drop shadow");
    assert!((shadow.opacity - 0.6).abs() < 0.01);
    assert!((shadow.blur - 10.0).abs() < 0.01);
    assert!((shadow.offset_x - 3.0).abs() < 0.01);
    assert!((shadow.offset_y + 5.0).abs() < 0.01);

    send_format(&mut editor, FormatMessage::ElementOpacity(80.0));
    send_format(&mut editor, FormatMessage::CommitInput);
    assert!((editor.format.body.opacity - 0.8).abs() < 0.01);
    assert!(editor.has_changes());
    assert_eq!(editor.song().format(), &editor.format);

    let _ = editor.update(Message::Undo);
    assert!((editor.format.body.opacity - 1.0).abs() < 0.01);
    let _ = editor.update(Message::Redo);
    assert!((editor.format.body.opacity - 0.8).abs() < 0.01);

    send_format(&mut editor, FormatMessage::ResetTarget);
    assert_eq!(editor.format.body, Default::default());
}

#[test]
fn text_tab_messages_update_typography_outline_shadow_bullets_and_layout() {
    let mut editor = editor("one");
    send_format(&mut editor, FormatMessage::TabSelected(FormatTab::Text));
    send_format(
        &mut editor,
        FormatMessage::TextSubTabSelected(TextSubTab::Style),
    );
    send_format(&mut editor, FormatMessage::TargetSelected(Target::Footer));
    send_format(&mut editor, FormatMessage::TargetSelected(Target::Body));

    send_format(
        &mut editor,
        FormatMessage::FontFamilyPicked(Some("Inter".into())),
    );
    assert_eq!(
        editor.format.body.text.font_family.as_deref(),
        Some("Inter")
    );

    send_format(
        &mut editor,
        FormatMessage::TextSizeMode(TextSizeMode::AutoFit),
    );
    assert_eq!(editor.format.body.text.font_size, Some(TextSize::AutoFit));
    send_format(
        &mut editor,
        FormatMessage::TextSizeMode(TextSizeMode::Fixed),
    );
    send_format(&mut editor, FormatMessage::TextSizeInput("44".into()));
    send_format(&mut editor, FormatMessage::TextSizeStep(4.0));
    assert_eq!(
        editor.format.body.text.font_size,
        Some(TextSize::Fixed(48.0))
    );
    send_format(
        &mut editor,
        FormatMessage::TextSizeMode(TextSizeMode::Theme),
    );
    assert_eq!(editor.format.body.text.font_size, None);

    send_format(&mut editor, FormatMessage::ToggleBold);
    send_format(&mut editor, FormatMessage::ToggleItalic);
    send_format(&mut editor, FormatMessage::ToggleUnderline);
    assert_eq!(editor.format.body.text.font_weight, Some(FontWeight::Bold));
    assert!(editor.format.body.text.italic);
    assert!(editor.format.body.text.underline);

    send_format(&mut editor, FormatMessage::StepIndent(12.0));
    assert!((editor.format.body.text.padding.left - 12.0).abs() < 0.01);
    send_format(&mut editor, FormatMessage::StepIndent(-20.0));
    assert!((editor.format.body.text.padding.left - 0.0).abs() < 0.01);

    send_format(&mut editor, FormatMessage::AlignX(Some(TextAlign::Left)));
    send_format(&mut editor, FormatMessage::AlignY(Some(VerticalAlign::Top)));
    assert_eq!(editor.format.body.text.align_x, Some(TextAlign::Left));
    assert_eq!(editor.format.body.text.align_y, Some(VerticalAlign::Top));

    send_format(&mut editor, FormatMessage::OutlineEnabled(true));
    send_format(&mut editor, FormatMessage::OutlineWidth("3".into()));
    assert_eq!(
        editor.format.body.text.outline,
        Some(TextOutline {
            width: 3.0,
            ..TextOutline::default()
        })
    );

    send_format(
        &mut editor,
        FormatMessage::TextShadowMode(TextShadowMode::Drop),
    );
    send_format(&mut editor, FormatMessage::TextShadowOpacity(75.0));
    send_format(&mut editor, FormatMessage::TextShadowOffsetX("2".into()));
    send_format(&mut editor, FormatMessage::TextShadowOffsetY("4".into()));
    match editor.format.body.text.shadow {
        Some(TextShadowOverride::Drop(shadow)) => {
            assert!((shadow.opacity - 0.75).abs() < 0.01);
            assert!((shadow.offset_x - 2.0).abs() < 0.01);
            assert!((shadow.offset_y - 4.0).abs() < 0.01);
        }
        other => panic!("unexpected text shadow: {other:?}"),
    }

    send_format(
        &mut editor,
        FormatMessage::BulletStyle(BulletStyle::Numbered),
    );
    assert_eq!(editor.format.body.text.bullets, BulletStyle::Numbered);

    send_format(
        &mut editor,
        FormatMessage::TextSubTabSelected(TextSubTab::Layout),
    );
    send_format(&mut editor, FormatMessage::ToggleLineHeightFixed(true));
    send_format(&mut editor, FormatMessage::LineHeightChanged(1.6));
    assert_eq!(editor.format.body.text.line_height, Some(1.6));

    send_format(
        &mut editor,
        FormatMessage::PaddingChanged(PaddingSide::Right, "24".into()),
    );
    send_format(
        &mut editor,
        FormatMessage::PaddingChanged(PaddingSide::Top, "16".into()),
    );
    send_format(
        &mut editor,
        FormatMessage::PaddingChanged(PaddingSide::Bottom, "8".into()),
    );
    assert!((editor.format.body.text.padding.right - 24.0).abs() < 0.01);
    assert!((editor.format.body.text.padding.top - 16.0).abs() < 0.01);
    assert!((editor.format.body.text.padding.bottom - 8.0).abs() < 0.01);
}

#[test]
fn arrange_tab_z_order_align_dimensions_and_lock() {
    let mut editor = editor("one");
    send_format(&mut editor, FormatMessage::TabSelected(FormatTab::Arrange));

    send_format(&mut editor, FormatMessage::TargetSelected(Target::Footer));
    send_format(&mut editor, FormatMessage::Order(OrderAction::Front));
    assert!(editor.format.footer_on_top);
    send_format(&mut editor, FormatMessage::Order(OrderAction::Back));
    assert!(!editor.format.footer_on_top);

    send_format(&mut editor, FormatMessage::TargetSelected(Target::Body));
    send_format(&mut editor, FormatMessage::Order(OrderAction::Back));
    assert!(editor.format.footer_on_top);
    send_format(&mut editor, FormatMessage::Order(OrderAction::Forward));
    assert!(!editor.format.footer_on_top);

    send_format(&mut editor, FormatMessage::Align(CanvasAlign::FullWidth));
    let full = editor.slides[0].body_box.expect("expected body box");
    assert!((full.x - 0.0).abs() < 0.001);
    assert!((full.width - 1.0).abs() < 0.001);

    let initial = SlideBox::new(0.1, 0.1, 0.5, 0.5);
    let _ = editor.update(Message::BoxChanged(Target::Body, initial));
    let _ = editor.update(Message::BoxCommitted);
    send_format(&mut editor, FormatMessage::Align(CanvasAlign::CenterBoth));
    let centered = editor.slides[0].body_box.expect("expected centered box");
    assert!(((centered.x + centered.width * 0.5) - 0.5).abs() < 0.001);
    assert!(((centered.y + centered.height * 0.5) - 0.5).abs() < 0.001);

    send_format(&mut editor, FormatMessage::ToggleConstrainProportions(true));
    let before = editor.slides[0]
        .body_box
        .expect("expected box before resize");
    let before_ratio = before.width / before.height;
    send_format(
        &mut editor,
        FormatMessage::DimChanged(DimField::Width, "640".into()),
    );
    let after = editor.slides[0].body_box.expect("expected resized box");
    assert!(((after.width / after.height) - before_ratio).abs() < 0.01);

    send_format(&mut editor, FormatMessage::SetLocked(true));
    assert!(editor.format.body.locked);
    let locked_box = editor.slides[0].body_box;

    let _ = editor.update(Message::BoxChanged(
        Target::Body,
        SlideBox::new(0.05, 0.05, 0.2, 0.2),
    ));
    send_format(&mut editor, FormatMessage::Align(CanvasAlign::Left));
    send_format(
        &mut editor,
        FormatMessage::DimChanged(DimField::Left, "100".into()),
    );
    let _ = editor.update(Message::ResetBox);
    assert_eq!(editor.slides[0].body_box, locked_box);

    paste_inline(&mut editor, "edited while locked");
    assert!(editor.slides[0].body.contains("edited while locked"));

    send_format(&mut editor, FormatMessage::SetLocked(false));
    let unlocked = SlideBox::new(0.05, 0.05, 0.2, 0.2);
    let _ = editor.update(Message::BoxChanged(Target::Body, unlocked));
    assert_eq!(editor.slides[0].body_box, Some(unlocked));
}

#[test]
fn toggle_format_shrinks_fit_scale_inside_max_modal_width() {
    let theme = SlideTheme::default();
    let mut editor = editor("one");
    let closed_scale = editor.fit_scale(&theme, super::MAX_MODAL_W, 720.0);
    let _ = editor.update(Message::ToggleFormat);
    assert!(editor.format_open);
    let open_scale = editor.fit_scale(&theme, super::MAX_MODAL_W, 720.0);
    assert!(open_scale < closed_scale);
    assert!(open_scale >= 0.12);
}

#[test]
fn selected_box_defaults_to_body_and_stays_visible_on_outside_click() {
    let mut editor = editor("one");
    assert_eq!(editor.selected_box, Some(Target::Body));

    let _ = editor.update(Message::BoxSelected(Some(Target::Footer)));
    assert_eq!(editor.selected_box, Some(Target::Footer));

    let _ = editor.update(Message::EditStarted(Target::Footer));
    assert_eq!(editor.editing, Some(Target::Footer));

    let _ = editor.update(Message::BoxSelected(None));
    assert!(editor.editing.is_none());
    assert_eq!(editor.selected_box, Some(Target::Footer));
}

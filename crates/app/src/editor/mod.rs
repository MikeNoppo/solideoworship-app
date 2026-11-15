mod canvas;
mod chrome;
mod format_panel;
mod history;
mod inline_editor;
pub mod reflow;
mod sidebar;
mod stage;

#[cfg(test)]
mod tests;

use iced::widget::{Space, column, container, row, text_editor};
use iced::{Element, Fill};
use sdw_core::format::SlideFormat;
use sdw_core::labels::LabelSettings;
use sdw_core::slide::{SlideBox, SlideDeck};
use sdw_core::theme::{SlideTheme, SongStyle};
use sdw_core::{Song, SongSlide, compile};
use sdw_library::SongId;

use self::canvas::{PaneSplitter, Target};
pub use self::format_panel::FormatMessage;
use self::format_panel::FormatState;
use self::history::{EditGroup, History, SlideDraft, Snapshot};
use self::reflow::{Reflow, Section};
use crate::i18n::{count, fill, t};
use crate::style;
use crate::ui;

pub const TITLE_INPUT: &str = "editor-title";
pub const LABEL_INPUT: &str = "editor-slide-label";
pub const CANVAS_INLINE_INPUT: &str = "editor-canvas-inline";
pub const COPYRIGHT_INLINE_INPUT: &str = "editor-copyright-inline";
pub const REFLOW_INPUT: &str = "editor-reflow";

const MIN_MODAL_W: f32 = 640.0;
const MAX_MODAL_W: f32 = 960.0;
const MIN_MODAL_H: f32 = 480.0;
const MAX_MODAL_H: f32 = 640.0;
const DEFAULT_SIDEBAR_W: f32 = 240.0;
const MIN_SIDEBAR_W: f32 = 180.0;
const MAX_SIDEBAR_W: f32 = 380.0;
const MIN_ZOOM: f32 = 0.12;
const MAX_ZOOM: f32 = 1.50;
const DEFAULT_ZOOM: f32 = 0.50;
const STAGE_PADDING: f32 = 16.0;
const CHROME_HEIGHT: f32 = 150.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Words,
    Slides,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(String),
    CopyrightChanged(String),
    TabSelected(Tab),
    SidebarResized(f32),
    SlideSelected(usize),
    SlideLabelChanged(String),
    AddSlide,
    DuplicateSlide,
    DeleteSlide,
    MoveSlideUp,
    MoveSlideDown,
    BoxSelected(Option<Target>),
    EditStarted(Target),
    FinishInlineEdit,
    InlineBodyEdited(text_editor::Action),
    BoxChanged(Target, SlideBox),
    BoxCommitted,
    ResetBox,
    ApplyBoxToAll,
    ToggleFormat,
    Format(FormatMessage),
    FocusBody,
    ZoomChanged(f32),
    ToggleZoomFit,
    ToggleReflow,
    ReflowEdited(text_editor::Action),
    ReflowInsert(Section),
    ApplyReflow,
    Undo,
    Redo,
    Apply,
    Save,
    Delete,
    DeleteConfirmed,
    DeleteCancelled,
    Cancel,
    Dismiss,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Focus(&'static str),
    Save {
        origin: Option<SongId>,
        song: Song,
        close: bool,
    },
    Delete(SongId),
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NoticeKind {
    BoxApplied,
    BoxAlreadyMatched,
    Saved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditorNotice {
    pub kind: NoticeKind,
    pub text: Box<str>,
}

pub struct Editor {
    origin: Option<SongId>,
    original: Song,
    title: String,
    copyright: String,
    slides: Vec<SlideDraft>,
    pub(super) customized: bool,
    pub(super) format: SlideFormat,
    pub(super) style: SongStyle,
    deck: SlideDeck,
    pub(super) selected: usize,
    tab: Tab,
    sidebar_width: f32,
    pub(super) selected_box: Option<Target>,
    pub(super) editing: Option<Target>,
    inline_body: text_editor::Content,
    zoom: f32,
    zoom_fit: bool,
    zoom_label: Box<str>,
    reflow: Option<Reflow>,
    pub(super) format_open: bool,
    format_ui: FormatState,
    history: History,
    pending_drag_snapshot: Option<Snapshot>,
    confirm_delete: bool,
    busy: bool,
    deleting: bool,
    error: Option<Box<str>>,
    pub(super) notice: Option<EditorNotice>,
    summary: Box<str>,
    ready: Box<str>,
    confirm: Box<str>,
}

impl Editor {
    pub fn with_style(origin: Option<SongId>, song: &Song, style: &SongStyle) -> Self {
        let (slides, customized) = match song.slides() {
            Some(custom) => (custom.iter().map(SlideDraft::from).collect(), true),
            None => {
                let compiled = compile::song_with_style(song, style);
                if compiled.slides.is_empty() {
                    (
                        vec![SlideDraft {
                            label: Some(t().editor.verse.into()),
                            body: Box::default(),
                            body_box: None,
                            footer_box: None,
                        }],
                        false,
                    )
                } else {
                    (
                        compiled
                            .slides
                            .into_iter()
                            .map(|slide| SlideDraft {
                                label: slide.label,
                                body: slide.body,
                                body_box: slide.body_box,
                                footer_box: slide.footer_box,
                            })
                            .collect(),
                        false,
                    )
                }
            }
        };

        let inline_body = slides
            .first()
            .map(|slide| text_editor::Content::with_text(&slide.body))
            .unwrap_or_default();

        let mut editor = Self {
            origin,
            original: song.clone(),
            title: song.title().to_owned(),
            copyright: song.copyright().unwrap_or_default().to_owned(),
            slides,
            customized,
            format: song.format().clone(),
            style: style.clone(),
            deck: compile::song_with_style(song, style),
            selected: 0,
            tab: Tab::Words,
            sidebar_width: DEFAULT_SIDEBAR_W,
            selected_box: Some(Target::Body),
            editing: None,
            inline_body,
            zoom: DEFAULT_ZOOM,
            zoom_fit: true,
            zoom_label: Box::default(),
            reflow: None,
            format_open: false,
            format_ui: FormatState::default(),
            history: History::default(),
            pending_drag_snapshot: None,
            confirm_delete: false,
            busy: false,
            deleting: false,
            error: None,
            notice: None,
            summary: Box::default(),
            ready: Box::default(),
            confirm: Box::default(),
        };
        editor.recompile();
        editor.update_zoom_label(DEFAULT_ZOOM);
        editor.sync_format_inputs();
        editor
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::TitleChanged(title) => {
                if title != self.title {
                    self.history.begin_group(EditGroup::Title, self.snapshot());
                    self.title = title;
                    self.notice = None;
                    self.recompile();
                }
            }
            Message::CopyrightChanged(copyright) => {
                if copyright != self.copyright {
                    self.history
                        .begin_group(EditGroup::Copyright, self.snapshot());
                    self.copyright = copyright;
                    self.notice = None;
                    self.recompile();
                }
            }
            Message::TabSelected(tab) => {
                self.tab = tab;
            }
            Message::SidebarResized(width) => {
                self.sidebar_width = width.clamp(MIN_SIDEBAR_W, MAX_SIDEBAR_W);
            }
            Message::SlideSelected(index) => {
                if index < self.slides.len() {
                    self.history.break_group();
                    self.selected = index;
                    self.editing = None;
                    self.notice = None;
                    self.sync_inline_body();
                    self.sync_format_inputs();
                }
            }
            Message::SlideLabelChanged(next) => {
                if let Some(slide) = self.slides.get(self.selected) {
                    let trimmed = next.trim();
                    let label = (!trimmed.is_empty()).then(|| Box::<str>::from(trimmed));
                    if slide.label != label {
                        let before = self.snapshot();
                        self.history
                            .begin_group(EditGroup::SlideLabel(self.selected), before);
                        self.slides[self.selected].label = label;
                        self.notice = None;
                        self.sync_customized();
                        self.recompile();
                    }
                }
            }
            Message::AddSlide => {
                self.push_history();
                let insert_at = (self.selected + 1).min(self.slides.len());
                self.slides.insert(
                    insert_at,
                    SlideDraft {
                        label: None,
                        body: Box::default(),
                        body_box: None,
                        footer_box: None,
                    },
                );
                self.selected = insert_at;
                self.sync_customized();
                self.editing = None;
                self.notice = None;
                self.sync_inline_body();
                self.sync_format_inputs();
                self.recompile();
            }
            Message::DuplicateSlide => {
                if let Some(current) = self.slides.get(self.selected).cloned() {
                    self.push_history();
                    let insert_at = self.selected + 1;
                    self.slides.insert(insert_at, current);
                    self.selected = insert_at;
                    self.sync_customized();
                    self.editing = None;
                    self.notice = None;
                    self.sync_inline_body();
                    self.sync_format_inputs();
                    self.recompile();
                }
            }
            Message::DeleteSlide => {
                if self.slides.len() > 1 && self.selected < self.slides.len() {
                    self.push_history();
                    self.slides.remove(self.selected);
                    self.selected = self.selected.min(self.slides.len().saturating_sub(1));
                    self.sync_customized();
                    self.editing = None;
                    self.notice = None;
                    self.sync_inline_body();
                    self.sync_format_inputs();
                    self.recompile();
                }
            }
            Message::MoveSlideUp => {
                if self.selected > 0 && self.selected < self.slides.len() {
                    self.push_history();
                    self.slides.swap(self.selected, self.selected - 1);
                    self.selected -= 1;
                    self.sync_customized();
                    self.editing = None;
                    self.notice = None;
                    self.recompile();
                }
            }
            Message::MoveSlideDown => {
                if self.selected + 1 < self.slides.len() {
                    self.push_history();
                    self.slides.swap(self.selected, self.selected + 1);
                    self.selected += 1;
                    self.sync_customized();
                    self.editing = None;
                    self.notice = None;
                    self.recompile();
                }
            }
            Message::BoxSelected(target) => {
                if self.editing != target {
                    self.editing = None;
                    self.history.break_group();
                }
                self.selected_box = target.or(self.selected_box).or(Some(Target::Body));
                self.sync_format_inputs();
            }
            Message::EditStarted(target) => {
                if self.slides.is_empty() {
                    return Action::None;
                }
                self.history.break_group();
                self.selected_box = Some(target);
                self.editing = Some(target);
                return match target {
                    Target::Body => {
                        self.sync_inline_body();
                        Action::Focus(CANVAS_INLINE_INPUT)
                    }
                    Target::Footer => Action::Focus(COPYRIGHT_INLINE_INPUT),
                };
            }
            Message::FinishInlineEdit => {
                self.editing = None;
                self.history.break_group();
            }
            Message::InlineBodyEdited(action) => {
                let is_edit = action.is_edit();
                self.inline_body.perform(action);
                if is_edit && self.selected < self.slides.len() {
                    let next_body = self.inline_body.text().into_boxed_str();
                    if self.slides[self.selected].body != next_body {
                        let before = self.snapshot();
                        self.history
                            .begin_group(EditGroup::SlideBody(self.selected), before);
                        self.slides[self.selected].body = next_body;
                        self.notice = None;
                        self.sync_customized();
                        self.recompile();
                    }
                }
            }
            Message::BoxChanged(target, rect) => {
                let locked = match target {
                    Target::Body => self.format.body.locked,
                    Target::Footer => self.format.footer.locked,
                };
                if locked {
                    return Action::None;
                }
                if self.pending_drag_snapshot.is_none() {
                    self.pending_drag_snapshot = Some(self.snapshot());
                }
                if let Some(slide) = self.slides.get_mut(self.selected) {
                    match target {
                        Target::Body => slide.body_box = Some(rect.clamped()),
                        Target::Footer => slide.footer_box = Some(rect.clamped()),
                    }
                    self.notice = None;
                    self.customized = true;
                    self.sync_format_inputs();
                    self.recompile();
                }
            }
            Message::BoxCommitted => {
                if let Some(snapshot) = self.pending_drag_snapshot.take() {
                    self.history.push(snapshot);
                }
            }
            Message::ResetBox => {
                if let Some(slide) = self.slides.get(self.selected) {
                    let reset_body = matches!(self.selected_box, Some(Target::Body) | None)
                        && slide.body_box.is_some()
                        && !self.format.body.locked;
                    let reset_footer = matches!(self.selected_box, Some(Target::Footer) | None)
                        && slide.footer_box.is_some()
                        && !self.format.footer.locked;
                    if reset_body || reset_footer {
                        self.push_history();
                        let slide = &mut self.slides[self.selected];
                        if reset_body {
                            slide.body_box = None;
                        }
                        if reset_footer {
                            slide.footer_box = None;
                        }
                        self.notice = None;
                        self.sync_customized();
                        self.sync_format_inputs();
                        self.recompile();
                    }
                }
            }
            Message::ApplyBoxToAll => {
                if let Some(current) = self.slides.get(self.selected).cloned() {
                    let differs = self.slides.iter().any(|slide| {
                        slide.body_box != current.body_box || slide.footer_box != current.footer_box
                    });
                    if differs {
                        self.push_history();
                        for slide in &mut self.slides {
                            slide.body_box = current.body_box;
                            slide.footer_box = current.footer_box;
                        }
                        self.sync_customized();
                        self.recompile();
                        self.notice = Some(EditorNotice {
                            kind: NoticeKind::BoxApplied,
                            text: count(
                                self.slides.len(),
                                t().editor.box_applied_one,
                                t().editor.box_applied_other,
                            ),
                        });
                    } else {
                        self.notice = Some(EditorNotice {
                            kind: NoticeKind::BoxAlreadyMatched,
                            text: t().editor.box_already_matched.into(),
                        });
                    }
                }
            }
            Message::ToggleFormat => {
                self.format_open = !self.format_open;
                if self.format_open {
                    self.sync_format_inputs();
                }
            }
            Message::Format(format_msg) => {
                self.apply_format(format_msg);
            }
            Message::FocusBody => {
                if !self.slides.is_empty() {
                    self.history.break_group();
                    self.selected_box = Some(Target::Body);
                    self.editing = Some(Target::Body);
                    self.sync_inline_body();
                    return Action::Focus(CANVAS_INLINE_INPUT);
                }
            }
            Message::ZoomChanged(zoom) => {
                self.zoom_fit = false;
                self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
                self.update_zoom_label(self.zoom);
            }
            Message::ToggleZoomFit => {
                self.zoom_fit = !self.zoom_fit;
                if !self.zoom_fit {
                    self.update_zoom_label(self.zoom);
                }
            }
            Message::ToggleReflow => {
                if self.reflow.is_some() {
                    self.reflow = None;
                } else {
                    self.editing = None;
                    let text = Song::derive_lyrics(&self.slides_as_song_slides());
                    self.reflow = Some(Reflow::new(
                        &text,
                        &self.title,
                        &self.copyright,
                        &self.style,
                    ));
                    return Action::Focus(REFLOW_INPUT);
                }
            }
            Message::ReflowEdited(action) => {
                if let Some(reflow) = &mut self.reflow {
                    let is_edit = action.is_edit();
                    reflow.lyrics.perform(action);
                    if is_edit {
                        reflow.sync_preview(&self.title, &self.copyright, &self.style);
                    }
                }
            }
            Message::ReflowInsert(section) => {
                if let Some(reflow) = &mut self.reflow {
                    reflow.insert(section);
                    reflow.sync_preview(&self.title, &self.copyright, &self.style);
                    return Action::Focus(REFLOW_INPUT);
                }
            }
            Message::ApplyReflow => {
                if let Some(reflow) = self.reflow.take() {
                    self.push_history();
                    let lyrics = reflow.lyrics.text();
                    let temp = Song::new(&self.title, &lyrics, Some(&self.copyright));
                    let compiled = compile::song_with_style(&temp, &self.style);
                    let mut next = Vec::with_capacity(compiled.slides.len().max(1));
                    for (index, item) in compiled.slides.into_iter().enumerate() {
                        let prev = self.slides.get(index);
                        next.push(SlideDraft {
                            label: item.label,
                            body: item.body,
                            body_box: prev.and_then(|slide| slide.body_box),
                            footer_box: prev.and_then(|slide| slide.footer_box),
                        });
                    }
                    if next.is_empty() {
                        next.push(SlideDraft {
                            label: Some(t().editor.verse.into()),
                            body: Box::default(),
                            body_box: None,
                            footer_box: None,
                        });
                    }
                    self.slides = next;
                    self.selected = self.selected.min(self.slides.len().saturating_sub(1));
                    self.sync_customized();
                    self.editing = None;
                    self.notice = None;
                    self.sync_inline_body();
                    self.sync_format_inputs();
                    self.recompile();
                }
            }
            Message::Undo => {
                if let Some(prev) = self.history.undo(self.snapshot()) {
                    self.restore(prev);
                }
            }
            Message::Redo => {
                if let Some(next) = self.history.redo(self.snapshot()) {
                    self.restore(next);
                }
            }
            Message::Apply => return self.save(false),
            Message::Save => return self.save(true),
            Message::Delete => {
                if self.origin.is_some() && !self.busy {
                    self.confirm_delete = true;
                    self.confirm = fill(
                        t().editor.confirm_delete,
                        &[("title", &self.original.title())],
                    )
                    .into_boxed_str();
                }
            }
            Message::DeleteCancelled => self.confirm_delete = false,
            Message::DeleteConfirmed => {
                if let Some(id) = self.origin.filter(|_| self.confirm_delete && !self.busy) {
                    self.busy = true;
                    self.deleting = true;
                    self.error = None;
                    return Action::Delete(id);
                }
            }
            Message::Cancel => return Action::Close,
            Message::Dismiss if self.confirm_delete && !self.busy => {
                self.confirm_delete = false;
            }
            Message::Dismiss if self.editing.is_some() => {
                self.editing = None;
                self.history.break_group();
            }
            Message::Dismiss if self.reflow.is_some() => {
                self.reflow = None;
            }
            Message::Dismiss => {
                if !self.busy && !self.has_changes() {
                    return Action::Close;
                }
            }
        }

        Action::None
    }

    pub fn name(&self) -> &str {
        self.original.title()
    }

    pub fn mark_applied(&mut self, id: SongId, song: &Song) {
        self.origin = Some(id);
        self.original = song.clone();
        self.busy = false;
        self.deleting = false;
        self.error = None;
        self.notice = Some(EditorNotice {
            kind: NoticeKind::Saved,
            text: t().editor.changes_saved.into(),
        });
    }

    pub fn fail(&mut self, error: Box<str>) {
        self.error = Some(if self.deleting {
            error
        } else {
            fill(
                t().editor.save_failed,
                &[("error", &error.trim_end_matches('.'))],
            )
            .into_boxed_str()
        });
        self.busy = false;
        self.deleting = false;
        self.confirm_delete = false;
    }

    pub fn view<'a>(
        &'a self,
        theme: &'a SlideTheme,
        labels: &'a LabelSettings,
        window_width: f32,
        window_height: f32,
    ) -> Element<'a, Message> {
        let modal_w = (window_width - 80.0).clamp(MIN_MODAL_W, MAX_MODAL_W);
        let modal_h = (window_height - 80.0).clamp(MIN_MODAL_H, MAX_MODAL_H);

        let body: Element<'_, Message> = if let Some(reflow) = &self.reflow {
            reflow.view()
        } else {
            let mut panes = row![
                self.sidebar(theme, labels),
                PaneSplitter::new(self.sidebar_width, Message::SidebarResized),
                self.stage_pane(theme, modal_w, modal_h),
            ];
            if self.format_open {
                panes = panes
                    .push(
                        container(Space::new())
                            .width(1)
                            .height(Fill)
                            .style(style::fill(style::BORDER)),
                    )
                    .push(self.format_panel(theme));
            }
            panes.height(Fill).into()
        };

        ui::sheet(
            column![
                self.top_bar(),
                ui::hrule(style::BORDER),
                body,
                ui::sheet_footer(self.footer()),
            ]
            .height(Fill),
            modal_w,
            modal_h,
        )
    }

    fn slides_as_song_slides(&self) -> Vec<SongSlide> {
        self.slides.iter().map(SongSlide::from).collect()
    }

    fn song(&self) -> Song {
        let slides = self.slides_as_song_slides();
        let song = if self.customized {
            Song::with_slides(&self.title, Some(&self.copyright), slides)
        } else {
            let lyrics = Song::derive_lyrics(&slides);
            Song::new(&self.title, &lyrics, Some(&self.copyright))
        };
        song.with_format(self.format.clone().clamped())
    }

    fn has_changes(&self) -> bool {
        let placeholder_only = self.slides.len() == 1
            && self.slides[0].body.trim().is_empty()
            && self.slides[0].body_box.is_none()
            && self.slides[0].footer_box.is_none()
            && self.format == *self.original.format();
        if placeholder_only {
            let clean_blank = self.origin.is_none()
                && self.title.trim().is_empty()
                && self.copyright.trim().is_empty();
            let clean_empty_original = self.original.slides().is_none()
                && self.original.lyrics().trim().is_empty()
                && self.title.trim() == self.original.title()
                && self.copyright.trim() == self.original.copyright().unwrap_or_default()
                && self.slides[0].label.as_deref() == Some(t().editor.verse);
            if clean_blank || clean_empty_original {
                return false;
            }
        }
        self.song() != self.original
    }

    pub(super) fn sync_customized(&mut self) {
        let has_boxes = self
            .slides
            .iter()
            .any(|slide| slide.body_box.is_some() || slide.footer_box.is_some());
        if has_boxes {
            self.customized = true;
            return;
        }
        let temp = Song::new(
            &self.title,
            &Song::derive_lyrics(&self.slides_as_song_slides()),
            Some(&self.copyright),
        );
        let auto = compile::song_with_style(&temp, &self.style);
        let matches_auto = auto.slides.len() == self.slides.len()
            && auto
                .slides
                .iter()
                .zip(&self.slides)
                .all(|(compiled, draft)| {
                    compiled.label == draft.label && compiled.body == draft.body
                });
        self.customized = !matches_auto;
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            title: self.title.clone(),
            copyright: self.copyright.clone(),
            slides: self.slides.clone(),
            selected: self.selected,
            customized: self.customized,
            format: self.format.clone(),
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.title = snapshot.title;
        self.copyright = snapshot.copyright;
        self.slides = snapshot.slides;
        self.selected = snapshot.selected.min(self.slides.len().saturating_sub(1));
        self.customized = snapshot.customized;
        self.format = snapshot.format;
        self.editing = None;
        self.notice = None;
        self.sync_inline_body();
        self.sync_format_inputs();
        self.recompile();
    }

    pub(super) fn push_history(&mut self) {
        self.history.push(self.snapshot());
    }

    fn sync_inline_body(&mut self) {
        let text = self
            .slides
            .get(self.selected)
            .map(|slide| slide.body.as_ref())
            .unwrap_or("");
        self.inline_body = text_editor::Content::with_text(text);
    }

    pub(super) fn sync_format_inputs(&mut self) {
        let theme = self.style.theme.clone();
        self.sync_format_inputs_with(&theme);
    }

    fn update_zoom_label(&mut self, zoom: f32) {
        let pct = (zoom * 100.0).round() as u16;
        self.zoom_label = format!("{pct}%").into_boxed_str();
    }

    pub(super) fn recompile(&mut self) {
        self.deck = compile::song_with_style(&self.song(), &self.style);
        if self.deck.slides.is_empty() && !self.slides.is_empty() {
            self.deck = compile::song_with_style(
                &Song::with_slides(
                    &self.title,
                    Some(&self.copyright),
                    self.slides_as_song_slides(),
                )
                .with_format(self.format.clone().clamped()),
                &self.style,
            );
        }
        self.selected = self.selected.min(self.slides.len().saturating_sub(1));
        self.describe();
    }

    fn describe(&mut self) {
        let slides = self.slides.len();
        let sections = self
            .slides
            .iter()
            .filter_map(|slide| slide.label.as_deref())
            .map(|label| label.split(" (").next().unwrap_or(label))
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let slides_text = count(slides, t().slides.count_one, t().slides.count_other);
        let line_count: usize = self
            .slides
            .iter()
            .map(|slide| slide.body.lines().count().max(1))
            .sum();

        self.summary = if sections == 0 {
            slides_text.clone()
        } else {
            format!(
                "{slides_text} · {}",
                count(sections, t().editor.sections_one, t().editor.sections_other)
            )
            .into_boxed_str()
        };
        self.ready = fill(
            t().editor.ready,
            &[
                (
                    "lines",
                    &count(line_count, t().editor.lines_one, t().editor.lines_other),
                ),
                ("slides", &slides_text),
            ],
        )
        .into_boxed_str();
    }

    fn save(&mut self, close: bool) -> Action {
        if self.busy {
            return Action::None;
        }

        let song = self.song();
        if song.title().is_empty() {
            self.error = Some(t().errors.empty_title.into());
            return Action::None;
        }

        self.busy = true;
        self.error = None;

        Action::Save {
            origin: self.origin,
            song,
            close,
        }
    }
}

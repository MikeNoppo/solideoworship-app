use std::sync::OnceLock;

use iced::widget::{button, center, column, container, row, scrollable, slider, text_input};
use iced::{Center, Element, Fill, Padding, Size, alignment};
use sdw_core::SlideBox;
use sdw_core::theme::{CANVAS_WIDTH, SlideTheme};

use super::canvas::{SlideCanvas, Target};
use super::format_panel::FORMAT_PANEL_W;
use super::inline_editor::AlignedEditor;
use super::{
    CANVAS_INLINE_INPUT, CHROME_HEIGHT, COPYRIGHT_INLINE_INPUT, Editor, MAX_ZOOM, MIN_ZOOM,
    Message, STAGE_PADDING,
};
use crate::i18n::t;
use crate::icon;
use crate::render::slide;
use crate::style::{self, Tone};
use crate::ui::{self, action, label};

impl Editor {
    pub(super) fn footer_visible_on_selected(&self) -> bool {
        self.style.show_copyright && (!self.style.copyright_first_slide_only || self.selected == 0)
    }

    pub(super) fn fit_scale(&self, theme: &SlideTheme, modal_w: f32, modal_h: f32) -> f32 {
        let format_w = if self.format_open {
            FORMAT_PANEL_W + 1.0
        } else {
            0.0
        };
        let avail_w = (modal_w
            - self.sidebar_width
            - crate::splitter::THICKNESS
            - format_w
            - STAGE_PADDING * 2.0)
            .max(160.0);
        let avail_h = (modal_h - CHROME_HEIGHT - STAGE_PADDING * 2.0).max(120.0);
        (avail_w / CANVAS_WIDTH)
            .min(avail_h / theme.layout.height())
            .clamp(MIN_ZOOM, MAX_ZOOM)
    }

    pub(super) fn stage_pane<'a>(
        &'a self,
        theme: &'a SlideTheme,
        modal_w: f32,
        modal_h: f32,
    ) -> Element<'a, Message> {
        let ratio = theme.layout.ratio();
        let fit_scale = self.fit_scale(theme, modal_w, modal_h);
        let scale = if self.zoom_fit { fit_scale } else { self.zoom };
        let canvas_w = (CANVAS_WIDTH * scale).round();
        let canvas_h = (canvas_w / ratio).round();
        let zoom_pct = (scale * 100.0).round() as u16;

        let canvas_area: Element<'_, Message> =
            if let Some(current_slide) = self.deck.slides.get(self.selected) {
                let body_rect = current_slide
                    .body_box
                    .unwrap_or_else(|| SlideBox::from_theme_body(theme));
                let footer_visible = self.footer_visible_on_selected();
                let footer_rect = if current_slide.footer.is_some()
                    || footer_visible
                    || self.editing == Some(Target::Footer)
                    || self.selected_box == Some(Target::Footer)
                {
                    Some(
                        current_slide
                            .footer_box
                            .unwrap_or_else(|| SlideBox::from_theme_footer(theme)),
                    )
                } else {
                    None
                };

                let body_override = if self.editing == Some(Target::Body) {
                    Some(slide::BoxText {
                        content: "",
                        alpha: 0.0,
                    })
                } else if current_slide.body.trim().is_empty() {
                    Some(slide::BoxText {
                        content: t().editor.double_click_to_edit,
                        alpha: 0.45,
                    })
                } else {
                    None
                };

                let footer_override = if self.editing == Some(Target::Footer) {
                    Some(slide::BoxText {
                        content: "",
                        alpha: 0.0,
                    })
                } else if current_slide.footer.is_none()
                    && footer_visible
                    && self.copyright.trim().is_empty()
                {
                    Some(slide::BoxText {
                        content: t().editor.copyright_hint,
                        alpha: 0.45,
                    })
                } else {
                    None
                };

                let backdrop = slide::editor_canvas(
                    current_slide,
                    theme,
                    canvas_w,
                    slide::BoxOverrides {
                        body: body_override,
                        footer: footer_override,
                    },
                );
                let inline_editor = self.inline_editor(theme, scale);

                let interactive = SlideCanvas::new(
                    backdrop,
                    inline_editor,
                    Size::new(canvas_w, canvas_h),
                    body_rect,
                    footer_rect,
                    self.format.body.locked,
                    self.format.footer.locked,
                    self.format.footer_on_top,
                    self.selected_box,
                    self.editing,
                    Message::BoxSelected,
                    Message::EditStarted,
                    Message::BoxChanged,
                    Message::BoxCommitted,
                );

                let stage_content = container(interactive).padding(1).style(style::boxed(
                    Some(style::SLIDE_BG),
                    style::BORDER_STRONG,
                    4.0,
                ));

                if self.zoom_fit {
                    center(stage_content).padding(STAGE_PADDING).into()
                } else {
                    scrollable(container(center(stage_content)).padding(STAGE_PADDING))
                        .direction(scrollable::Direction::Both {
                            vertical: scrollable::Scrollbar::new().width(4).scroller_width(4),
                            horizontal: scrollable::Scrollbar::new().width(4).scroller_width(4),
                        })
                        .width(Fill)
                        .height(Fill)
                        .style(style::scrollbar)
                        .into()
                }
            } else {
                center(ui::empty(
                    icon::FILE_TEXT,
                    false,
                    t().editor.no_slides,
                    Some(t().editor.no_slides_body),
                ))
                .into()
            };

        let overflow_note = self
            .deck
            .slides
            .get(self.selected)
            .is_some_and(|slide| slide::overflows(slide, theme))
            .then(|| ui::note(icon::CIRCLE_ALERT, t().editor.overflow_warning, style::LIVE));

        let done_edit = self.editing.is_some().then(|| {
            action(
                Tone::Secondary,
                Some(icon::CHECK),
                t().editor.done_editing,
                None,
                Some(Message::FinishInlineEdit),
            )
            .height(26)
        });

        let zoom_bar = container(
            row![
                container(
                    row![]
                        .push(overflow_note)
                        .push(done_edit)
                        .spacing(12)
                        .align_y(Center),
                )
                .width(Fill),
                button(label(
                    t().editor.zoom_fit,
                    11.5,
                    style::MEDIUM,
                    if self.zoom_fit {
                        style::TEXT
                    } else {
                        style::TEXT_3
                    },
                ))
                .height(24)
                .padding([0, 8])
                .on_press(Message::ToggleZoomFit)
                .style(style::pressable(
                    if self.zoom_fit {
                        Tone::Selected
                    } else {
                        Tone::Outline
                    },
                    5.0,
                )),
                label(
                    if self.zoom_fit {
                        zoom_pct_text(zoom_pct)
                    } else {
                        self.zoom_label.as_ref()
                    },
                    11.5,
                    style::MEDIUM,
                    style::TEXT_2,
                )
                .width(38),
                container(slider(MIN_ZOOM..=MAX_ZOOM, scale, Message::ZoomChanged).step(0.01_f32))
                    .width(130),
            ]
            .spacing(10)
            .align_y(Center),
        )
        .padding([6, 16])
        .width(Fill)
        .style(style::fill(style::SURFACE));

        column![
            container(canvas_area).width(Fill).height(Fill),
            ui::hrule(style::BORDER),
            zoom_bar,
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn inline_editor(&self, theme: &SlideTheme, scale: f32) -> Option<Element<'_, Message>> {
        let target = self.editing?;
        let current_slide = self.deck.slides.get(self.selected)?;
        let is_footer = target == Target::Footer;
        let box_rect = if is_footer {
            current_slide
                .footer_box
                .unwrap_or_else(|| SlideBox::from_theme_footer(theme))
        } else {
            current_slide
                .body_box
                .unwrap_or_else(|| SlideBox::from_theme_body(theme))
        };
        let resolved = slide::resolved_box_text_style(current_slide, is_footer, theme, box_rect);
        let text_color = slide::color_with_opacity(resolved.color, resolved.opacity.max(0.2));
        let size = (resolved.size * scale).max(10.0);

        match target {
            Target::Body => {
                let body = self
                    .slides
                    .get(self.selected)
                    .map(|slide| slide.body.as_ref())
                    .unwrap_or("");
                let padding = Padding {
                    top: resolved.padding.top * scale,
                    right: resolved.padding.right * scale,
                    bottom: resolved.padding.bottom * scale,
                    left: resolved.padding.left * scale,
                };
                Some(
                    AlignedEditor::new(
                        CANVAS_INLINE_INPUT,
                        &self.inline_body,
                        body,
                        t().editor.double_click_to_edit,
                        resolved.font,
                        size.into(),
                        resolved.line_height.into(),
                        resolved.align_x,
                        resolved.align_y,
                        padding,
                        style::canvas_text_editor(text_color),
                        Message::InlineBodyEdited,
                    )
                    .into(),
                )
            }
            Target::Footer => {
                let padding = Padding {
                    top: (resolved.padding.top * scale).max(2.0),
                    right: (resolved.padding.right * scale).max(4.0),
                    bottom: (resolved.padding.bottom * scale).max(2.0),
                    left: (resolved.padding.left * scale).max(4.0),
                };
                let input = text_input(t().editor.copyright_placeholder, &self.copyright)
                    .id(COPYRIGHT_INLINE_INPUT)
                    .on_input(Message::CopyrightChanged)
                    .on_submit(Message::FinishInlineEdit)
                    .font(resolved.font)
                    .align_x(alignment::Horizontal::from(resolved.align_x))
                    .size(size)
                    .padding(padding)
                    .style(style::canvas_text_input(text_color));
                Some(
                    container(input)
                        .align_y(resolved.align_y)
                        .width(Fill)
                        .height(Fill)
                        .into(),
                )
            }
        }
    }
}

fn zoom_pct_text(pct: u16) -> &'static str {
    static TABLE: OnceLock<Vec<Box<str>>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        (0..=200)
            .map(|value| format!("{value}%").into_boxed_str())
            .collect()
    });
    table
        .get(usize::from(pct))
        .map(AsRef::as_ref)
        .unwrap_or("100%")
}

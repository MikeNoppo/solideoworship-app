use iced::widget::{
    button, checkbox, column, container, row, scrollable, slider, space, stack, text, text_input,
};
use iced::{Center, Element, Fill, Padding, alignment};
use sdw_core::format::{
    BORDER_WIDTH, BOX_PADDING, BoxBorder, BoxFill, BoxFormat, BoxShadow, BulletStyle,
    CORNER_RADIUS, OPACITY, OUTLINE_WIDTH, SHADOW_BLUR, SHADOW_OFFSET, SlideFormat, TextOutline,
    TextShadow, TextShadowOverride, TextSize, VerticalAlign,
};
use sdw_core::slide::{MIN_BOX_SIZE, SlideBox};
use sdw_core::theme::{
    BODY_SIZE, CANVAS_WIDTH, FOOTER_SIZE, FontSpec, FontWeight, LINE_HEIGHT, Rgb, SlideTheme,
    TextAlign,
};

use super::canvas::Target;
use super::history::EditGroup;
use super::{Editor, Message};
use crate::i18n::t;
use crate::icon;
use crate::render::{fonts, slide};
use crate::style::{self, Tone};
use crate::ui::{action, label};

pub(super) const FORMAT_PANEL_W: f32 = 252.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatTab {
    #[default]
    Style,
    Text,
    Arrange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextSubTab {
    #[default]
    Style,
    Layout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Fill,
    Border,
    BoxShadow,
    Font,
    Outline,
    TextShadow,
    Bullets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    Default,
    None,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderMode {
    None,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxShadowMode {
    None,
    Drop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSizeMode {
    Theme,
    AutoFit,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextShadowMode {
    Theme,
    None,
    Drop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSlot {
    Fill,
    Border,
    BoxShadow,
    Text,
    Outline,
    TextShadow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderAction {
    Back,
    Front,
    Backward,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasAlign {
    Left,
    CenterH,
    Right,
    Top,
    CenterV,
    Bottom,
    CenterBoth,
    FitSafe,
    FullWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimField {
    Left,
    Top,
    Width,
    Height,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone)]
pub enum FormatMessage {
    TargetSelected(Target),
    TabSelected(FormatTab),
    TextSubTabSelected(TextSubTab),
    ToggleSection(SectionKind),
    ResetTarget,
    FillMode(FillMode),
    FillOpacity(f32),
    FillOpacityInput(String),
    BorderMode(BorderMode),
    BorderWidth(String),
    BorderRadius(String),
    BoxShadowMode(BoxShadowMode),
    BoxShadowOpacity(f32),
    BoxShadowOpacityInput(String),
    BoxShadowBlur(String),
    BoxShadowOffsetX(String),
    BoxShadowOffsetY(String),
    ElementOpacity(f32),
    ElementOpacityInput(String),
    ColorHexChanged(ColorSlot, String),
    ColorPresetPicked(ColorSlot, Rgb),
    ResetTextColor,
    ToggleFontPicker,
    FontQueryChanged(String),
    FontFamilyPicked(Option<String>),
    TextSizeMode(TextSizeMode),
    TextSizeInput(String),
    TextSizeStep(f32),
    ToggleBold,
    ToggleItalic,
    ToggleUnderline,
    StepIndent(f32),
    AlignX(Option<TextAlign>),
    AlignY(Option<VerticalAlign>),
    OutlineEnabled(bool),
    OutlineWidth(String),
    TextShadowMode(TextShadowMode),
    TextShadowOpacity(f32),
    TextShadowOpacityInput(String),
    TextShadowOffsetX(String),
    TextShadowOffsetY(String),
    BulletStyle(BulletStyle),
    ToggleLineHeightFixed(bool),
    LineHeightChanged(f32),
    LineHeightInput(String),
    PaddingChanged(PaddingSide, String),
    Order(OrderAction),
    Align(CanvasAlign),
    DimChanged(DimField, String),
    ToggleConstrainProportions(bool),
    SetLocked(bool),
    CommitInput,
}

#[derive(Debug, Clone)]
pub(super) struct FormatState {
    pub tab: FormatTab,
    pub text_subtab: TextSubTab,
    pub fill_open: bool,
    pub border_open: bool,
    pub box_shadow_open: bool,
    pub font_open: bool,
    pub outline_open: bool,
    pub text_shadow_open: bool,
    pub bullets_open: bool,
    pub font_picker_open: bool,
    pub font_query: String,
    pub constrain_proportions: bool,
    pub fill_hex: String,
    pub fill_opacity: String,
    pub border_hex: String,
    pub border_width: String,
    pub border_radius: String,
    pub box_shadow_hex: String,
    pub box_shadow_opacity: String,
    pub box_shadow_blur: String,
    pub box_shadow_x: String,
    pub box_shadow_y: String,
    pub element_opacity: String,
    pub font_size: String,
    pub text_hex: String,
    pub outline_hex: String,
    pub outline_width: String,
    pub text_shadow_hex: String,
    pub text_shadow_opacity: String,
    pub text_shadow_x: String,
    pub text_shadow_y: String,
    pub line_height: String,
    pub pad_left: String,
    pub pad_right: String,
    pub pad_top: String,
    pub pad_bottom: String,
    pub dim_left: String,
    pub dim_top: String,
    pub dim_width: String,
    pub dim_height: String,
}

impl Default for FormatState {
    fn default() -> Self {
        Self {
            tab: FormatTab::Style,
            text_subtab: TextSubTab::Style,
            fill_open: true,
            border_open: true,
            box_shadow_open: true,
            font_open: true,
            outline_open: false,
            text_shadow_open: false,
            bullets_open: false,
            font_picker_open: false,
            font_query: String::new(),
            constrain_proportions: false,
            fill_hex: String::new(),
            fill_opacity: String::new(),
            border_hex: String::new(),
            border_width: String::new(),
            border_radius: String::new(),
            box_shadow_hex: String::new(),
            box_shadow_opacity: String::new(),
            box_shadow_blur: String::new(),
            box_shadow_x: String::new(),
            box_shadow_y: String::new(),
            element_opacity: String::new(),
            font_size: String::new(),
            text_hex: String::new(),
            outline_hex: String::new(),
            outline_width: String::new(),
            text_shadow_hex: String::new(),
            text_shadow_opacity: String::new(),
            text_shadow_x: String::new(),
            text_shadow_y: String::new(),
            line_height: String::new(),
            pad_left: String::new(),
            pad_right: String::new(),
            pad_top: String::new(),
            pad_bottom: String::new(),
            dim_left: String::new(),
            dim_top: String::new(),
            dim_width: String::new(),
            dim_height: String::new(),
        }
    }
}

impl FormatState {
    pub fn sync_from(
        &mut self,
        fmt: &BoxFormat,
        rect: SlideBox,
        canvas_h: f32,
        default_size: f32,
        default_lh: f32,
        default_text_color: Rgb,
    ) {
        let (fill_color, fill_alpha) = match fmt.fill {
            Some(BoxFill::Solid { color, opacity }) => (color, opacity),
            _ => (Rgb::new(0.0, 0.0, 0.0), 0.7),
        };
        self.fill_hex = fill_color.to_hex();
        self.fill_opacity = pct_string(fill_alpha);

        let border = fmt.border.unwrap_or_default();
        self.border_hex = border.color.to_hex();
        self.border_width = round_string(border.width);
        self.border_radius = round_string(border.radius);

        let bshadow = fmt.shadow.unwrap_or_default();
        self.box_shadow_hex = bshadow.color.to_hex();
        self.box_shadow_opacity = pct_string(bshadow.opacity);
        self.box_shadow_blur = round_string(bshadow.blur);
        self.box_shadow_x = round_string(bshadow.offset_x);
        self.box_shadow_y = round_string(bshadow.offset_y);

        self.element_opacity = pct_string(fmt.opacity);

        let text = &fmt.text;
        let resolved_size = match text.font_size {
            Some(TextSize::Fixed(px)) => px,
            _ => default_size,
        };
        self.font_size = round_string(resolved_size);
        self.text_hex = text.color.unwrap_or(default_text_color).to_hex();

        let outline = text.outline.unwrap_or_default();
        self.outline_hex = outline.color.to_hex();
        self.outline_width = round_string(outline.width);

        let tshadow = match text.shadow {
            Some(TextShadowOverride::Drop(s)) => s,
            _ => TextShadow::default(),
        };
        self.text_shadow_hex = tshadow.color.to_hex();
        self.text_shadow_opacity = pct_string(tshadow.opacity);
        self.text_shadow_x = round_string(tshadow.offset_x);
        self.text_shadow_y = round_string(tshadow.offset_y);

        self.line_height = format!("{:.2}", text.line_height.unwrap_or(default_lh));
        self.pad_left = round_string(text.padding.left);
        self.pad_right = round_string(text.padding.right);
        self.pad_top = round_string(text.padding.top);
        self.pad_bottom = round_string(text.padding.bottom);

        self.dim_left = round_string(rect.x * CANVAS_WIDTH);
        self.dim_top = round_string(rect.y * canvas_h);
        self.dim_width = round_string(rect.width * CANVAS_WIDTH);
        self.dim_height = round_string(rect.height * canvas_h);
    }

    pub fn toggle_section(&mut self, kind: SectionKind) {
        let target = match kind {
            SectionKind::Fill => &mut self.fill_open,
            SectionKind::Border => &mut self.border_open,
            SectionKind::BoxShadow => &mut self.box_shadow_open,
            SectionKind::Font => &mut self.font_open,
            SectionKind::Outline => &mut self.outline_open,
            SectionKind::TextShadow => &mut self.text_shadow_open,
            SectionKind::Bullets => &mut self.bullets_open,
        };
        *target = !*target;
    }
}

pub(super) fn box_format_mut(format: &mut SlideFormat, target: Target) -> &mut BoxFormat {
    match target {
        Target::Body => &mut format.body,
        Target::Footer => &mut format.footer,
    }
}

pub(super) fn box_format(format: &SlideFormat, target: Target) -> &BoxFormat {
    match target {
        Target::Body => &format.body,
        Target::Footer => &format.footer,
    }
}

pub(super) fn pct_string(fraction: f32) -> String {
    ((fraction * 100.0).round() as i32).to_string()
}

fn round_string(value: f32) -> String {
    (value.round() as i32).to_string()
}

pub(super) fn align_box_on_canvas(
    rect: SlideBox,
    theme: &SlideTheme,
    target: Target,
    align: CanvasAlign,
) -> SlideBox {
    let rect = rect.clamped();
    match align {
        CanvasAlign::Left => SlideBox::new(0.0, rect.y, rect.width, rect.height),
        CanvasAlign::CenterH => {
            SlideBox::new((1.0 - rect.width) * 0.5, rect.y, rect.width, rect.height)
        }
        CanvasAlign::Right => SlideBox::new(1.0 - rect.width, rect.y, rect.width, rect.height),
        CanvasAlign::Top => SlideBox::new(rect.x, 0.0, rect.width, rect.height),
        CanvasAlign::CenterV => {
            SlideBox::new(rect.x, (1.0 - rect.height) * 0.5, rect.width, rect.height)
        }
        CanvasAlign::Bottom => SlideBox::new(rect.x, 1.0 - rect.height, rect.width, rect.height),
        CanvasAlign::CenterBoth => SlideBox::new(
            (1.0 - rect.width) * 0.5,
            (1.0 - rect.height) * 0.5,
            rect.width,
            rect.height,
        ),
        CanvasAlign::FitSafe => match target {
            Target::Body => SlideBox::from_theme_body(theme),
            Target::Footer => SlideBox::from_theme_footer(theme),
        },
        CanvasAlign::FullWidth => SlideBox::new(0.0, rect.y, 1.0, rect.height),
    }
}

impl Editor {
    pub(super) fn format_target(&self) -> Target {
        self.selected_box.unwrap_or(Target::Body)
    }

    pub(super) fn active_box_rect(&self, theme: &SlideTheme, target: Target) -> SlideBox {
        let current = self.slides.get(self.selected);
        match target {
            Target::Body => current
                .and_then(|s| s.body_box)
                .unwrap_or_else(|| SlideBox::from_theme_body(theme)),
            Target::Footer => current
                .and_then(|s| s.footer_box)
                .unwrap_or_else(|| SlideBox::from_theme_footer(theme)),
        }
    }

    pub(super) fn sync_format_inputs_with(&mut self, theme: &SlideTheme) {
        let target = self.format_target();
        let rect = self.active_box_rect(theme, target);
        let fmt = box_format(&self.format, target);
        let (default_size, default_lh, default_color) = match target {
            Target::Body => (theme.body_size, theme.line_height, theme.body),
            Target::Footer => (theme.footer_size, 1.3, theme.footer),
        };
        self.format_ui.sync_from(
            fmt,
            rect,
            theme.layout.height(),
            default_size,
            default_lh,
            default_color,
        );
    }

    pub(super) fn format_panel<'a>(&'a self, theme: &'a SlideTheme) -> Element<'a, Message> {
        let target = self.format_target();
        let fmt = box_format(&self.format, target);
        let ui = &self.format_ui;
        let copy = &t().format;
        let msg = |m: FormatMessage| Message::Format(m);

        let target_bar = row![
            segmented_btn(
                copy.target_body,
                target == Target::Body,
                !self.busy,
                msg(FormatMessage::TargetSelected(Target::Body)),
            ),
            segmented_btn(
                copy.target_footer,
                target == Target::Footer,
                !self.busy,
                msg(FormatMessage::TargetSelected(Target::Footer)),
            ),
            space::horizontal(),
            action(
                Tone::Ghost,
                Some(icon::ROTATE_CCW),
                copy.reset,
                None,
                (!self.busy && *fmt != BoxFormat::default())
                    .then_some(msg(FormatMessage::ResetTarget)),
            )
            .height(24),
        ]
        .spacing(4)
        .align_y(Center);

        let tabs = row![
            tab_btn(
                copy.tab_style,
                ui.tab == FormatTab::Style,
                msg(FormatMessage::TabSelected(FormatTab::Style)),
            ),
            tab_btn(
                copy.tab_text,
                ui.tab == FormatTab::Text,
                msg(FormatMessage::TabSelected(FormatTab::Text)),
            ),
            tab_btn(
                copy.tab_arrange,
                ui.tab == FormatTab::Arrange,
                msg(FormatMessage::TabSelected(FormatTab::Arrange)),
            ),
        ]
        .spacing(2);

        let tab_body = match ui.tab {
            FormatTab::Style => self.style_tab(fmt, target),
            FormatTab::Text => self.text_tab(fmt, theme, target),
            FormatTab::Arrange => self.arrange_tab(fmt, target, theme),
        };

        let scroll = scrollable(container(tab_body).padding(10).width(Fill))
            .height(Fill)
            .style(style::scrollbar);

        container(
            column![
                container(target_bar).padding([6, 8]),
                crate::ui::hrule(style::BORDER),
                container(tabs).padding([4, 8]),
                crate::ui::hrule(style::BORDER),
                scroll,
            ]
            .height(Fill),
        )
        .width(FORMAT_PANEL_W)
        .height(Fill)
        .style(style::fill(style::SURFACE))
        .into()
    }

    fn style_tab<'a>(&'a self, fmt: &'a BoxFormat, target: Target) -> Element<'a, Message> {
        let ui = &self.format_ui;
        let copy = &t().format;
        let busy = self.busy;
        let msg = Message::Format;

        let fill_mode = match fmt.fill {
            None => FillMode::Default,
            Some(BoxFill::None) => FillMode::None,
            Some(BoxFill::Solid { .. }) => FillMode::Solid,
        };
        let mut fill_col = column![
            row![
                segmented_btn(
                    copy.default,
                    fill_mode == FillMode::Default,
                    !busy,
                    msg(FormatMessage::FillMode(FillMode::Default)),
                ),
                segmented_btn(
                    copy.none,
                    fill_mode == FillMode::None,
                    !busy,
                    msg(FormatMessage::FillMode(FillMode::None)),
                ),
                segmented_btn(
                    copy.solid,
                    fill_mode == FillMode::Solid,
                    !busy,
                    msg(FormatMessage::FillMode(FillMode::Solid)),
                ),
            ]
            .spacing(4)
        ]
        .spacing(8);

        if let Some(BoxFill::Solid { color, opacity }) = fmt.fill {
            fill_col = fill_col
                .push(color_block(
                    copy.color,
                    &ui.fill_hex,
                    color,
                    ColorSlot::Fill,
                    busy,
                    None,
                ))
                .push(opacity_row(
                    copy.opacity,
                    opacity,
                    &ui.fill_opacity,
                    busy,
                    move |v| msg(FormatMessage::FillOpacity(v)),
                    move |s| msg(FormatMessage::FillOpacityInput(s)),
                ));
        }

        let border_mode = if fmt.border.is_some() {
            BorderMode::Solid
        } else {
            BorderMode::None
        };
        let mut border_col = column![
            row![
                segmented_btn(
                    copy.none,
                    border_mode == BorderMode::None,
                    !busy,
                    msg(FormatMessage::BorderMode(BorderMode::None)),
                ),
                segmented_btn(
                    copy.solid,
                    border_mode == BorderMode::Solid,
                    !busy,
                    msg(FormatMessage::BorderMode(BorderMode::Solid)),
                ),
            ]
            .spacing(4)
        ]
        .spacing(8);

        if let Some(border) = fmt.border {
            border_col = border_col
                .push(color_block(
                    copy.color,
                    &ui.border_hex,
                    border.color,
                    ColorSlot::Border,
                    busy,
                    None,
                ))
                .push(
                    row![
                        labeled_input(copy.width, &ui.border_width, "px", busy, move |s| msg(
                            FormatMessage::BorderWidth(s)
                        ),),
                        labeled_input(copy.radius, &ui.border_radius, "px", busy, move |s| msg(
                            FormatMessage::BorderRadius(s)
                        ),),
                    ]
                    .spacing(6),
                );
        }

        let shadow_mode = if fmt.shadow.is_some() {
            BoxShadowMode::Drop
        } else {
            BoxShadowMode::None
        };
        let mut shadow_col = column![
            row![
                segmented_btn(
                    copy.none,
                    shadow_mode == BoxShadowMode::None,
                    !busy,
                    msg(FormatMessage::BoxShadowMode(BoxShadowMode::None)),
                ),
                segmented_btn(
                    copy.drop,
                    shadow_mode == BoxShadowMode::Drop,
                    !busy,
                    msg(FormatMessage::BoxShadowMode(BoxShadowMode::Drop)),
                ),
            ]
            .spacing(4)
        ]
        .spacing(8);

        if let Some(bshadow) = fmt.shadow {
            shadow_col = shadow_col
                .push(color_block(
                    copy.color,
                    &ui.box_shadow_hex,
                    bshadow.color,
                    ColorSlot::BoxShadow,
                    busy,
                    None,
                ))
                .push(opacity_row(
                    copy.opacity,
                    bshadow.opacity,
                    &ui.box_shadow_opacity,
                    busy,
                    move |v| msg(FormatMessage::BoxShadowOpacity(v)),
                    move |s| msg(FormatMessage::BoxShadowOpacityInput(s)),
                ))
                .push(
                    row![
                        labeled_input(copy.blur, &ui.box_shadow_blur, "px", busy, move |s| {
                            msg(FormatMessage::BoxShadowBlur(s))
                        }),
                        labeled_input(copy.offset_x, &ui.box_shadow_x, "px", busy, move |s| {
                            msg(FormatMessage::BoxShadowOffsetX(s))
                        }),
                        labeled_input(copy.offset_y, &ui.box_shadow_y, "px", busy, move |s| {
                            msg(FormatMessage::BoxShadowOffsetY(s))
                        }),
                    ]
                    .spacing(4),
                );
        }

        let elem_opacity = column![
            label(copy.element_opacity, 11.0, style::SEMIBOLD, style::TEXT_3),
            opacity_row(
                copy.opacity,
                fmt.opacity,
                &ui.element_opacity,
                busy,
                move |v| msg(FormatMessage::ElementOpacity(v)),
                move |s| msg(FormatMessage::ElementOpacityInput(s)),
            ),
        ]
        .spacing(6);

        let _ = target;
        column![
            collapsible_section(copy.fill, ui.fill_open, SectionKind::Fill, fill_col.into(),),
            crate::ui::hrule(style::BORDER),
            collapsible_section(
                copy.border,
                ui.border_open,
                SectionKind::Border,
                border_col.into(),
            ),
            crate::ui::hrule(style::BORDER),
            collapsible_section(
                copy.shadow,
                ui.box_shadow_open,
                SectionKind::BoxShadow,
                shadow_col.into(),
            ),
            crate::ui::hrule(style::BORDER),
            elem_opacity,
        ]
        .spacing(10)
        .into()
    }

    fn text_tab<'a>(
        &'a self,
        fmt: &'a BoxFormat,
        theme: &'a SlideTheme,
        target: Target,
    ) -> Element<'a, Message> {
        let ui = &self.format_ui;
        let copy = &t().format;
        let msg = Message::Format;

        let subtabs = row![
            segmented_btn(
                copy.subtab_style,
                ui.text_subtab == TextSubTab::Style,
                true,
                msg(FormatMessage::TextSubTabSelected(TextSubTab::Style)),
            ),
            segmented_btn(
                copy.subtab_layout,
                ui.text_subtab == TextSubTab::Layout,
                true,
                msg(FormatMessage::TextSubTabSelected(TextSubTab::Layout)),
            ),
        ]
        .spacing(4);

        let body = match ui.text_subtab {
            TextSubTab::Style => self.text_style_subtab(fmt, theme, target),
            TextSubTab::Layout => self.text_layout_subtab(fmt, theme),
        };

        column![subtabs, body].spacing(10).into()
    }

    fn text_style_subtab<'a>(
        &'a self,
        fmt: &'a BoxFormat,
        theme: &'a SlideTheme,
        target: Target,
    ) -> Element<'a, Message> {
        let ui = &self.format_ui;
        let copy = &t().format;
        let busy = self.busy;
        let tf = &fmt.text;
        let msg = Message::Format;

        let default_font = match target {
            Target::Body => &theme.body_font,
            Target::Footer => &theme.footer_font,
        };
        let default_color = match target {
            Target::Body => theme.body,
            Target::Footer => theme.footer,
        };
        let default_align_y = VerticalAlign::default();

        let family_label = tf.font_family.as_deref().unwrap_or(&default_font.family);
        let family_btn = button(
            row![
                text(family_label)
                    .size(12)
                    .color(style::TEXT)
                    .font(fonts::resolve(&font_spec(family_label))),
                space::horizontal(),
                icon::view(icon::CHEVRON_DOWN, 12.0, style::TEXT_3),
            ]
            .align_y(Center),
        )
        .on_press_maybe((!busy).then_some(msg(FormatMessage::ToggleFontPicker)))
        .padding([5, 8])
        .width(Fill)
        .style(style::pressable(Tone::Secondary, 6.0));

        let mut font_col = column![family_btn].spacing(8);

        if ui.font_picker_open {
            let mut picker = column![
                button(label(
                    copy.use_theme_font,
                    11.5,
                    style::MEDIUM,
                    if tf.font_family.is_none() {
                        style::TEXT
                    } else {
                        style::TEXT_2
                    },
                ))
                .on_press_maybe((!busy).then_some(msg(FormatMessage::FontFamilyPicked(None))))
                .padding([4, 6])
                .width(Fill)
                .style(style::pressable(
                    if tf.font_family.is_none() {
                        Tone::Selected
                    } else {
                        Tone::Row
                    },
                    4.0,
                )),
                text_input(t().settings.font_search, &ui.font_query)
                    .on_input(move |s| msg(FormatMessage::FontQueryChanged(s)))
                    .on_submit(msg(FormatMessage::CommitInput))
                    .size(12)
                    .padding([4, 8])
                    .style(style::field),
            ]
            .spacing(4);

            let matches = fonts::search(&ui.font_query, 7);
            let has_more = matches.len() > 6;
            if matches.is_empty() {
                picker = picker.push(label(
                    t().settings.font_empty,
                    11.5,
                    style::INTER,
                    style::TEXT_3,
                ));
            } else {
                for family in matches.into_iter().take(6) {
                    let selected = tf.font_family.as_deref() == Some(family);
                    picker = picker.push(
                        button(
                            text(family)
                                .size(12)
                                .color(style::TEXT)
                                .font(fonts::resolve(&font_spec(family))),
                        )
                        .on_press_maybe((!busy).then_some(msg(FormatMessage::FontFamilyPicked(
                            Some(family.to_owned()),
                        ))))
                        .padding([4, 6])
                        .width(Fill)
                        .style(style::pressable(
                            if selected { Tone::Selected } else { Tone::Row },
                            4.0,
                        )),
                    );
                }
                if has_more {
                    picker = picker.push(label(
                        t().settings.font_limit,
                        10.5,
                        style::INTER,
                        style::TEXT_3,
                    ));
                }
            }

            font_col = font_col.push(container(picker).padding(6).width(Fill).style(style::boxed(
                Some(style::BG),
                style::BORDER_STRONG,
                6.0,
            )));
        }

        let size_mode = match tf.font_size {
            None => TextSizeMode::Theme,
            Some(TextSize::AutoFit) => TextSizeMode::AutoFit,
            Some(TextSize::Fixed(_)) => TextSizeMode::Fixed,
        };
        let mut size_row = row![
            segmented_btn(
                copy.theme,
                size_mode == TextSizeMode::Theme,
                !busy,
                msg(FormatMessage::TextSizeMode(TextSizeMode::Theme)),
            ),
            segmented_btn(
                copy.auto,
                size_mode == TextSizeMode::AutoFit,
                !busy,
                msg(FormatMessage::TextSizeMode(TextSizeMode::AutoFit)),
            ),
            segmented_btn(
                copy.fixed,
                size_mode == TextSizeMode::Fixed,
                !busy,
                msg(FormatMessage::TextSizeMode(TextSizeMode::Fixed)),
            ),
        ]
        .spacing(4)
        .align_y(Center);
        if size_mode == TextSizeMode::Fixed {
            size_row = size_row.push(
                text_input("", &ui.font_size)
                    .on_input_maybe(
                        (!busy).then_some(move |s| msg(FormatMessage::TextSizeInput(s))),
                    )
                    .on_submit(msg(FormatMessage::CommitInput))
                    .font(style::MONO)
                    .size(11.5)
                    .padding([4, 6])
                    .width(44)
                    .style(style::field),
            );
        }
        let size_step = row![
            label(copy.size, 11.5, style::INTER, style::TEXT_2),
            space::horizontal(),
            button(label(copy.size_down, 11.5, style::SEMIBOLD, style::TEXT))
                .on_press_maybe((!busy).then_some(msg(FormatMessage::TextSizeStep(-2.0))))
                .padding([3, 8])
                .style(style::pressable(Tone::Secondary, 5.0)),
            button(label(copy.size_up, 11.5, style::SEMIBOLD, style::TEXT))
                .on_press_maybe((!busy).then_some(msg(FormatMessage::TextSizeStep(2.0))))
                .padding([3, 8])
                .style(style::pressable(Tone::Secondary, 5.0)),
        ]
        .spacing(4)
        .align_y(Center);

        let text_color = tf.color.unwrap_or(default_color);
        let bold_active = tf.font_weight.unwrap_or(default_font.weight) == FontWeight::Bold;

        let biu_indent = row![
            toggle_glyph_btn(
                copy.bold_key,
                style::BOLD,
                bold_active,
                !busy,
                msg(FormatMessage::ToggleBold),
            ),
            toggle_glyph_btn(
                copy.italic_key,
                style::MEDIUM,
                tf.italic,
                !busy,
                msg(FormatMessage::ToggleItalic),
            ),
            toggle_glyph_btn(
                copy.underline_key,
                style::MEDIUM,
                tf.underline,
                !busy,
                msg(FormatMessage::ToggleUnderline),
            ),
            space::horizontal(),
            label(copy.indent, 11.0, style::INTER, style::TEXT_3),
            button(icon::view(icon::MINUS, 11.0, style::TEXT))
                .on_press_maybe(
                    (!busy && tf.padding.left > 0.0)
                        .then_some(msg(FormatMessage::StepIndent(-20.0))),
                )
                .padding([4, 6])
                .style(style::pressable(Tone::Secondary, 5.0)),
            button(icon::view(icon::PLUS, 11.0, style::TEXT))
                .on_press_maybe((!busy).then_some(msg(FormatMessage::StepIndent(20.0))))
                .padding([4, 6])
                .style(style::pressable(Tone::Secondary, 5.0)),
        ]
        .spacing(3)
        .align_y(Center);

        let cur_ax = tf.align_x;
        let cur_ay = tf.align_y.unwrap_or(default_align_y);
        let align_row = row![
            label(copy.align, 11.0, style::INTER, style::TEXT_3),
            space::horizontal(),
            icon_toggle(
                icon::TEXT_ALIGN_START,
                cur_ax == Some(TextAlign::Left),
                !busy,
                msg(FormatMessage::AlignX(if cur_ax == Some(TextAlign::Left) {
                    None
                } else {
                    Some(TextAlign::Left)
                })),
            ),
            icon_toggle(
                icon::TEXT_ALIGN_CENTER,
                cur_ax == Some(TextAlign::Center),
                !busy,
                msg(FormatMessage::AlignX(
                    if cur_ax == Some(TextAlign::Center) {
                        None
                    } else {
                        Some(TextAlign::Center)
                    }
                )),
            ),
            icon_toggle(
                icon::TEXT_ALIGN_END,
                cur_ax == Some(TextAlign::Right),
                !busy,
                msg(FormatMessage::AlignX(if cur_ax == Some(TextAlign::Right) {
                    None
                } else {
                    Some(TextAlign::Right)
                })),
            ),
            crate::ui::vrule(16.0),
            valign_btn(
                VerticalAlign::Top,
                cur_ay == VerticalAlign::Top,
                !busy,
                msg(FormatMessage::AlignY(Some(VerticalAlign::Top))),
            ),
            valign_btn(
                VerticalAlign::Middle,
                cur_ay == VerticalAlign::Middle,
                !busy,
                msg(FormatMessage::AlignY(Some(VerticalAlign::Middle))),
            ),
            valign_btn(
                VerticalAlign::Bottom,
                cur_ay == VerticalAlign::Bottom,
                !busy,
                msg(FormatMessage::AlignY(Some(VerticalAlign::Bottom))),
            ),
        ]
        .spacing(2)
        .align_y(Center);

        font_col = font_col
            .push(size_step)
            .push(size_row)
            .push(color_block(
                copy.color,
                &ui.text_hex,
                text_color,
                ColorSlot::Text,
                busy,
                tf.color.map(|_| msg(FormatMessage::ResetTextColor)),
            ))
            .push(biu_indent)
            .push(align_row);

        let has_outline = tf.outline.is_some();
        let mut outline_col = column![
            row![
                segmented_btn(
                    copy.none,
                    !has_outline,
                    !busy,
                    msg(FormatMessage::OutlineEnabled(false)),
                ),
                segmented_btn(
                    copy.solid,
                    has_outline,
                    !busy,
                    msg(FormatMessage::OutlineEnabled(true)),
                ),
            ]
            .spacing(4)
        ]
        .spacing(8);

        if let Some(TextOutline { color, .. }) = tf.outline {
            outline_col = outline_col
                .push(color_block(
                    copy.color,
                    &ui.outline_hex,
                    color,
                    ColorSlot::Outline,
                    busy,
                    None,
                ))
                .push(labeled_input(
                    copy.width,
                    &ui.outline_width,
                    "px",
                    busy,
                    move |s| msg(FormatMessage::OutlineWidth(s)),
                ));
        }

        let tshadow_mode = match tf.shadow {
            None => TextShadowMode::Theme,
            Some(TextShadowOverride::None) => TextShadowMode::None,
            Some(TextShadowOverride::Drop(_)) => TextShadowMode::Drop,
        };
        let mut tshadow_col = column![
            row![
                segmented_btn(
                    copy.theme,
                    tshadow_mode == TextShadowMode::Theme,
                    !busy,
                    msg(FormatMessage::TextShadowMode(TextShadowMode::Theme)),
                ),
                segmented_btn(
                    copy.none,
                    tshadow_mode == TextShadowMode::None,
                    !busy,
                    msg(FormatMessage::TextShadowMode(TextShadowMode::None)),
                ),
                segmented_btn(
                    copy.drop,
                    tshadow_mode == TextShadowMode::Drop,
                    !busy,
                    msg(FormatMessage::TextShadowMode(TextShadowMode::Drop)),
                ),
            ]
            .spacing(4)
        ]
        .spacing(8);

        if let Some(TextShadowOverride::Drop(ts)) = tf.shadow {
            tshadow_col = tshadow_col
                .push(color_block(
                    copy.color,
                    &ui.text_shadow_hex,
                    ts.color,
                    ColorSlot::TextShadow,
                    busy,
                    None,
                ))
                .push(opacity_row(
                    copy.opacity,
                    ts.opacity,
                    &ui.text_shadow_opacity,
                    busy,
                    move |v| msg(FormatMessage::TextShadowOpacity(v)),
                    move |s| msg(FormatMessage::TextShadowOpacityInput(s)),
                ))
                .push(
                    row![
                        labeled_input(copy.offset_x, &ui.text_shadow_x, "px", busy, move |s| {
                            msg(FormatMessage::TextShadowOffsetX(s))
                        }),
                        labeled_input(copy.offset_y, &ui.text_shadow_y, "px", busy, move |s| {
                            msg(FormatMessage::TextShadowOffsetY(s))
                        }),
                    ]
                    .spacing(6),
                );
        }

        let bullets_col = row![
            segmented_btn(
                copy.none,
                tf.bullets == BulletStyle::None,
                !busy,
                msg(FormatMessage::BulletStyle(BulletStyle::None)),
            ),
            segmented_btn(
                copy.disc,
                tf.bullets == BulletStyle::Disc,
                !busy,
                msg(FormatMessage::BulletStyle(BulletStyle::Disc)),
            ),
            segmented_btn(
                copy.numbered,
                tf.bullets == BulletStyle::Numbered,
                !busy,
                msg(FormatMessage::BulletStyle(BulletStyle::Numbered)),
            ),
        ]
        .spacing(4);

        column![
            collapsible_section(copy.font, ui.font_open, SectionKind::Font, font_col.into()),
            crate::ui::hrule(style::BORDER),
            collapsible_section(
                copy.outline,
                ui.outline_open,
                SectionKind::Outline,
                outline_col.into(),
            ),
            crate::ui::hrule(style::BORDER),
            collapsible_section(
                copy.shadow,
                ui.text_shadow_open,
                SectionKind::TextShadow,
                tshadow_col.into(),
            ),
            crate::ui::hrule(style::BORDER),
            collapsible_section(
                copy.bullets,
                ui.bullets_open,
                SectionKind::Bullets,
                bullets_col.into(),
            ),
        ]
        .spacing(10)
        .into()
    }

    fn text_layout_subtab<'a>(
        &'a self,
        fmt: &'a BoxFormat,
        theme: &'a SlideTheme,
    ) -> Element<'a, Message> {
        let ui = &self.format_ui;
        let copy = &t().format;
        let busy = self.busy;
        let tf = &fmt.text;
        let msg = Message::Format;
        let custom_lh = tf.line_height.is_some();

        let mut lh_col = column![
            label(copy.line_spacing, 11.0, style::SEMIBOLD, style::TEXT_3),
            row![
                segmented_btn(
                    copy.theme,
                    !custom_lh,
                    !busy,
                    msg(FormatMessage::ToggleLineHeightFixed(false)),
                ),
                segmented_btn(
                    copy.fixed,
                    custom_lh,
                    !busy,
                    msg(FormatMessage::ToggleLineHeightFixed(true)),
                ),
            ]
            .spacing(4),
        ]
        .spacing(6);

        if let Some(lh) = tf.line_height {
            lh_col = lh_col.push(
                row![
                    slider(0.8..=2.5, lh.clamp(0.8, 2.5), move |v| {
                        msg(FormatMessage::LineHeightChanged(v))
                    })
                    .step(0.05_f32)
                    .width(Fill),
                    text_input("", &ui.line_height)
                        .on_input_maybe(
                            (!busy).then_some(move |s| msg(FormatMessage::LineHeightInput(s))),
                        )
                        .on_submit(msg(FormatMessage::CommitInput))
                        .font(style::MONO)
                        .size(11.5)
                        .padding([4, 6])
                        .width(48)
                        .style(style::field),
                ]
                .spacing(6)
                .align_y(Center),
            );
        } else {
            lh_col = lh_col.push(
                text(format!("{:.2}", theme.line_height))
                    .size(11.5)
                    .font(style::MONO)
                    .color(style::TEXT_3),
            );
        }

        let padding_col = column![
            label(copy.inner_padding, 11.0, style::SEMIBOLD, style::TEXT_3),
            row![
                labeled_input(t().settings.left, &ui.pad_left, "px", busy, move |s| {
                    msg(FormatMessage::PaddingChanged(PaddingSide::Left, s))
                }),
                labeled_input(t().settings.right, &ui.pad_right, "px", busy, move |s| {
                    msg(FormatMessage::PaddingChanged(PaddingSide::Right, s))
                }),
            ]
            .spacing(6),
            row![
                labeled_input(t().settings.top, &ui.pad_top, "px", busy, move |s| {
                    msg(FormatMessage::PaddingChanged(PaddingSide::Top, s))
                }),
                labeled_input(t().settings.bottom, &ui.pad_bottom, "px", busy, move |s| {
                    msg(FormatMessage::PaddingChanged(PaddingSide::Bottom, s))
                }),
            ]
            .spacing(6),
        ]
        .spacing(8);

        column![lh_col, crate::ui::hrule(style::BORDER), padding_col]
            .spacing(12)
            .into()
    }

    fn arrange_tab<'a>(
        &'a self,
        fmt: &'a BoxFormat,
        target: Target,
        theme: &'a SlideTheme,
    ) -> Element<'a, Message> {
        let ui = &self.format_ui;
        let copy = &t().format;
        let busy = self.busy;
        let locked = fmt.locked;
        let msg = Message::Format;

        let is_in_front = match target {
            Target::Body => !self.format.footer_on_top,
            Target::Footer => self.format.footer_on_top,
        };
        let can_back = !busy && is_in_front;
        let can_front = !busy && !is_in_front;

        let z_order = column![
            row![
                order_btn(
                    copy.send_to_back,
                    can_back,
                    msg(FormatMessage::Order(OrderAction::Back)),
                ),
                order_btn(
                    copy.bring_to_front,
                    can_front,
                    msg(FormatMessage::Order(OrderAction::Front)),
                ),
            ]
            .spacing(4),
            row![
                order_btn(
                    copy.send_backward,
                    can_back,
                    msg(FormatMessage::Order(OrderAction::Backward)),
                ),
                order_btn(
                    copy.bring_forward,
                    can_front,
                    msg(FormatMessage::Order(OrderAction::Forward)),
                ),
            ]
            .spacing(4),
        ]
        .spacing(4);

        let align_grid = column![
            label(copy.canvas_align, 11.0, style::SEMIBOLD, style::TEXT_3),
            row![
                align_glyph_btn(CanvasAlign::Left, !busy && !locked),
                align_glyph_btn(CanvasAlign::CenterH, !busy && !locked),
                align_glyph_btn(CanvasAlign::Right, !busy && !locked),
                crate::ui::vrule(16.0),
                align_glyph_btn(CanvasAlign::Top, !busy && !locked),
                align_glyph_btn(CanvasAlign::CenterV, !busy && !locked),
                align_glyph_btn(CanvasAlign::Bottom, !busy && !locked),
            ]
            .spacing(3)
            .align_y(Center),
            row![
                align_glyph_btn(CanvasAlign::CenterBoth, !busy && !locked),
                align_glyph_btn(CanvasAlign::FitSafe, !busy && !locked),
                align_glyph_btn(CanvasAlign::FullWidth, !busy && !locked),
            ]
            .spacing(3),
        ]
        .spacing(6);

        let dim_disabled = busy || locked;
        let dims = column![
            label(copy.dimensions, 11.0, style::SEMIBOLD, style::TEXT_3),
            row![
                labeled_input(
                    t().settings.left,
                    &ui.dim_left,
                    "px",
                    dim_disabled,
                    move |s| msg(FormatMessage::DimChanged(DimField::Left, s)),
                ),
                labeled_input(copy.width, &ui.dim_width, "px", dim_disabled, move |s| {
                    msg(FormatMessage::DimChanged(DimField::Width, s))
                }),
            ]
            .spacing(6),
            row![
                labeled_input(
                    t().settings.top,
                    &ui.dim_top,
                    "px",
                    dim_disabled,
                    move |s| msg(FormatMessage::DimChanged(DimField::Top, s)),
                ),
                labeled_input(copy.height, &ui.dim_height, "px", dim_disabled, move |s| {
                    msg(FormatMessage::DimChanged(DimField::Height, s))
                }),
            ]
            .spacing(6),
            checkbox(ui.constrain_proportions)
                .label(copy.constrain_proportions)
                .size(14)
                .text_size(12)
                .on_toggle_maybe(
                    (!busy).then_some(move |v| msg(FormatMessage::ToggleConstrainProportions(v))),
                ),
        ]
        .spacing(8);

        let lock_row = row![
            segmented_btn(
                copy.lock,
                locked,
                !busy && !locked,
                msg(FormatMessage::SetLocked(true)),
            ),
            segmented_btn(
                copy.unlock,
                !locked,
                !busy && locked,
                msg(FormatMessage::SetLocked(false)),
            ),
        ]
        .spacing(6);

        let _ = theme;
        let mut col = column![
            z_order,
            crate::ui::hrule(style::BORDER),
            align_grid,
            crate::ui::hrule(style::BORDER),
            dims,
            crate::ui::hrule(style::BORDER),
            lock_row,
        ]
        .spacing(10);

        if locked {
            col = col.push(text(copy.locked_hint).size(11.0).color(style::LOCKED));
        }

        col.into()
    }
}

fn collapsible_section<'a>(
    title: &'a str,
    open: bool,
    kind: SectionKind,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let header = button(
        row![
            icon::view(
                if open {
                    icon::CHEVRON_DOWN
                } else {
                    icon::ARROW_RIGHT
                },
                11.0,
                style::TEXT_3,
            ),
            label(title, 11.0, style::SEMIBOLD, style::TEXT_3),
            space::horizontal(),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .on_press(Message::Format(FormatMessage::ToggleSection(kind)))
    .padding([3, 2])
    .width(Fill)
    .style(style::pressable(Tone::Ghost, 4.0));

    if open {
        column![header, body].spacing(8).into()
    } else {
        header.into()
    }
}

fn tab_btn<'a>(text_str: &'a str, active: bool, on_press: Message) -> Element<'a, Message> {
    button(
        container(label(
            text_str,
            12.0,
            if active {
                style::SEMIBOLD
            } else {
                style::MEDIUM
            },
            if active { style::TEXT } else { style::TEXT_2 },
        ))
        .center_x(Fill),
    )
    .on_press(on_press)
    .padding([5, 6])
    .width(Fill)
    .style(style::pressable(
        if active { Tone::Selected } else { Tone::Ghost },
        6.0,
    ))
    .into()
}

fn segmented_btn<'a>(
    text_str: &'a str,
    active: bool,
    enabled: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        container(label(
            text_str,
            11.0,
            if active { style::MEDIUM } else { style::INTER },
            if active {
                style::TEXT
            } else {
                style::fade(style::TEXT_2, enabled)
            },
        ))
        .center_x(Fill),
    )
    .on_press_maybe(enabled.then_some(on_press))
    .padding([4, 6])
    .width(Fill)
    .style(style::pressable(
        if active {
            Tone::Selected
        } else {
            Tone::Secondary
        },
        5.0,
    ))
    .into()
}

fn order_btn<'a>(text_str: &'a str, enabled: bool, on_press: Message) -> Element<'a, Message> {
    button(
        container(label(
            text_str,
            11.0,
            style::MEDIUM,
            style::fade(style::TEXT, enabled),
        ))
        .center_x(Fill),
    )
    .on_press_maybe(enabled.then_some(on_press))
    .padding([4, 6])
    .width(Fill)
    .style(style::pressable(Tone::Secondary, 5.0))
    .into()
}

fn toggle_glyph_btn<'a>(
    glyph: &'a str,
    font: iced::Font,
    active: bool,
    enabled: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        container(text(glyph).font(font).size(12).color(if active {
            style::TEXT
        } else {
            style::TEXT_2
        }))
        .center_x(Fill),
    )
    .on_press_maybe(enabled.then_some(on_press))
    .padding([4, 8])
    .style(style::pressable(
        if active {
            Tone::Toggled
        } else {
            Tone::Secondary
        },
        5.0,
    ))
    .into()
}

fn icon_toggle<'a>(
    glyph: &'a str,
    active: bool,
    enabled: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(icon::view(
        glyph,
        12.0,
        if active { style::TEXT } else { style::TEXT_2 },
    ))
    .on_press_maybe(enabled.then_some(on_press))
    .padding([5, 6])
    .style(style::pressable(
        if active {
            Tone::Toggled
        } else {
            Tone::Secondary
        },
        5.0,
    ))
    .into()
}

fn valign_btn<'a>(
    align: VerticalAlign,
    active: bool,
    enabled: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let fg = if active { style::TEXT } else { style::TEXT_2 };
    let bar = |w: f32| {
        container(space::horizontal())
            .width(w)
            .height(1.5)
            .style(style::fill(fg))
    };
    let bars = column![bar(10.0), bar(6.0), bar(10.0)]
        .spacing(1.5)
        .align_x(Center);
    let inner = container(bars)
        .width(14)
        .height(14)
        .center_x(14)
        .align_y(match align {
            VerticalAlign::Top => alignment::Vertical::Top,
            VerticalAlign::Middle => alignment::Vertical::Center,
            VerticalAlign::Bottom => alignment::Vertical::Bottom,
        });

    button(inner)
        .on_press_maybe(enabled.then_some(on_press))
        .padding([4, 5])
        .style(style::pressable(
            if active {
                Tone::Toggled
            } else {
                Tone::Secondary
            },
            5.0,
        ))
        .into()
}

fn align_glyph_btn<'a>(align: CanvasAlign, enabled: bool) -> Element<'a, Message> {
    let fg = style::fade(style::TEXT_2, enabled);
    let accent = style::fade(style::TEXT, enabled);
    let line_v = || {
        container(space::vertical())
            .width(1.5)
            .height(12)
            .style(style::fill(fg))
    };
    let line_h = || {
        container(space::horizontal())
            .width(12)
            .height(1.5)
            .style(style::fill(fg))
    };
    let box_sm = |w: f32, h: f32| {
        container(space::horizontal())
            .width(w)
            .height(h)
            .style(style::fill(accent))
    };

    let diagram: Element<'a, Message> = match align {
        CanvasAlign::Left => row![
            line_v(),
            column![box_sm(8.0, 3.0), box_sm(5.0, 3.0)].spacing(2)
        ]
        .spacing(2)
        .align_y(Center)
        .into(),
        CanvasAlign::CenterH => column![box_sm(10.0, 3.0), box_sm(6.0, 3.0)]
            .spacing(2)
            .align_x(Center)
            .into(),
        CanvasAlign::Right => row![
            column![box_sm(8.0, 3.0), box_sm(5.0, 3.0)]
                .spacing(2)
                .align_x(alignment::Horizontal::Right),
            line_v(),
        ]
        .spacing(2)
        .align_y(Center)
        .into(),
        CanvasAlign::Top => column![
            line_h(),
            row![box_sm(3.0, 8.0), box_sm(3.0, 5.0)].spacing(2),
        ]
        .spacing(2)
        .align_x(Center)
        .into(),
        CanvasAlign::CenterV => row![box_sm(3.0, 10.0), box_sm(3.0, 6.0)]
            .spacing(2)
            .align_y(Center)
            .into(),
        CanvasAlign::Bottom => column![
            row![box_sm(3.0, 8.0), box_sm(3.0, 5.0)]
                .spacing(2)
                .align_y(alignment::Vertical::Bottom),
            line_h(),
        ]
        .spacing(2)
        .align_x(Center)
        .into(),
        CanvasAlign::CenterBoth => container(box_sm(6.0, 6.0))
            .width(14)
            .height(12)
            .center_x(14)
            .center_y(12)
            .style(style::boxed(None, fg, 2.0))
            .into(),
        CanvasAlign::FitSafe => container(box_sm(9.0, 7.0))
            .width(14)
            .height(12)
            .center_x(14)
            .center_y(12)
            .style(style::boxed(None, fg, 2.0))
            .into(),
        CanvasAlign::FullWidth => row![line_v(), box_sm(8.0, 5.0), line_v()]
            .spacing(1.5)
            .align_y(Center)
            .into(),
    };

    button(
        container(diagram)
            .width(16)
            .height(14)
            .center_x(16)
            .center_y(14),
    )
    .on_press_maybe(enabled.then_some(Message::Format(FormatMessage::Align(align))))
    .padding([4, 5])
    .style(style::pressable(Tone::Secondary, 5.0))
    .into()
}

fn color_block<'a>(
    caption: &'a str,
    hex_value: &'a str,
    current: Rgb,
    slot: ColorSlot,
    busy: bool,
    reset_msg: Option<Message>,
) -> Element<'a, Message> {
    let hex_field = stack![
        text_input("#rrggbb", hex_value)
            .on_input_maybe(
                (!busy)
                    .then_some(move |s| Message::Format(FormatMessage::ColorHexChanged(slot, s))),
            )
            .on_submit(Message::Format(FormatMessage::CommitInput))
            .font(style::MONO)
            .size(11.5)
            .padding(Padding {
                top: 4.0,
                right: 6.0,
                bottom: 4.0,
                left: 26.0,
            })
            .style(if Rgb::from_hex(hex_value).is_none() {
                style::invalid_field
            } else {
                style::field
            }),
        container(
            container(space::horizontal())
                .width(14)
                .height(14)
                .style(style::sample(slide::color(current)))
        )
        .padding(Padding::ZERO.left(6))
        .center_y(Fill),
    ]
    .width(Fill)
    .height(26);

    let mut top = row![label(caption, 11.5, style::INTER, style::TEXT_2), hex_field,]
        .spacing(8)
        .align_y(Center);

    if let Some(reset) = reset_msg {
        top = top.push(
            button(icon::view(icon::ROTATE_CCW, 11.0, style::TEXT_2))
                .on_press_maybe((!busy).then_some(reset))
                .padding([4, 5])
                .style(style::pressable(Tone::Ghost, 5.0)),
        );
    }

    let swatches = style::COLOR_PRESETS
        .into_iter()
        .fold(row![].spacing(4), |list, preset| {
            list.push(
                button(space::horizontal())
                    .width(16)
                    .height(16)
                    .padding(0)
                    .on_press_maybe((!busy).then_some(Message::Format(
                        FormatMessage::ColorPresetPicked(slot, preset),
                    )))
                    .style(style::swatch(slide::color(preset), preset == current)),
            )
        });

    column![top, swatches].spacing(6).into()
}

fn opacity_row<'a>(
    caption: &'a str,
    value: f32,
    input_value: &'a str,
    busy: bool,
    on_slide: impl Fn(f32) -> Message + 'a,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let pct = (value.clamp(0.0, 1.0) * 100.0).round();
    row![
        label(caption, 11.0, style::INTER, style::TEXT_2),
        slider(0.0..=100.0, pct, on_slide).step(1.0_f32).width(Fill),
        text_input("100", input_value)
            .on_input_maybe((!busy).then_some(on_input))
            .on_submit(Message::Format(FormatMessage::CommitInput))
            .font(style::MONO)
            .size(11.5)
            .padding([3, 5])
            .width(38)
            .style(style::field),
        label("%", 11.0, style::INTER, style::TEXT_3),
    ]
    .spacing(5)
    .align_y(Center)
    .into()
}

fn labeled_input<'a>(
    caption: &'a str,
    value: &'a str,
    unit: &'a str,
    disabled: bool,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    column![
        label(caption, 10.5, style::INTER, style::TEXT_3),
        row![
            text_input("", value)
                .on_input_maybe((!disabled).then_some(on_input))
                .on_submit(Message::Format(FormatMessage::CommitInput))
                .font(style::MONO)
                .size(11.5)
                .padding([4, 6])
                .width(Fill)
                .style(style::field),
            label(unit, 10.5, style::INTER, style::TEXT_3),
        ]
        .spacing(3)
        .align_y(Center),
    ]
    .spacing(3)
    .width(Fill)
    .into()
}

impl Editor {
    pub(super) fn apply_format(&mut self, msg: FormatMessage) {
        let target = self.format_target();
        let theme = self.style.theme.clone();
        let size_range = match target {
            Target::Body => &BODY_SIZE,
            Target::Footer => &FOOTER_SIZE,
        };
        let default_size = match target {
            Target::Body => theme.body_size,
            Target::Footer => theme.footer_size,
        };
        let default_lh = match target {
            Target::Body => theme.line_height,
            Target::Footer => 1.3,
        };

        match msg {
            FormatMessage::TargetSelected(next) => {
                self.selected_box = Some(next);
                self.editing = None;
                self.history.break_group();
                self.format_ui.font_picker_open = false;
                self.sync_format_inputs();
            }
            FormatMessage::TabSelected(tab) => {
                self.format_ui.tab = tab;
                self.format_ui.font_picker_open = false;
            }
            FormatMessage::TextSubTabSelected(sub) => {
                self.format_ui.text_subtab = sub;
                self.format_ui.font_picker_open = false;
            }
            FormatMessage::ToggleSection(kind) => {
                self.format_ui.toggle_section(kind);
            }
            FormatMessage::ToggleFontPicker => {
                self.format_ui.font_picker_open = !self.format_ui.font_picker_open;
            }
            FormatMessage::FontQueryChanged(query) => {
                self.format_ui.font_query = query;
            }
            FormatMessage::ToggleConstrainProportions(constrain) => {
                self.format_ui.constrain_proportions = constrain;
            }
            FormatMessage::CommitInput => {
                self.history.break_group();
                self.sync_format_inputs();
            }
            FormatMessage::ResetTarget => {
                self.commit_format_discrete(|fmt| *fmt = BoxFormat::default());
            }
            FormatMessage::FillMode(mode) => {
                self.commit_format_discrete(|fmt| {
                    fmt.fill = match mode {
                        FillMode::Default => None,
                        FillMode::None => Some(BoxFill::None),
                        FillMode::Solid => Some(BoxFill::Solid {
                            color: Rgb::new(0.0, 0.0, 0.0),
                            opacity: 0.7,
                        }),
                    };
                });
            }
            FormatMessage::FillOpacity(pct) => {
                let alpha = pct_to_opacity(pct);
                self.format_ui.fill_opacity = pct_string(alpha);
                self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                    if let Some(BoxFill::Solid { opacity, .. }) = &mut fmt.fill {
                        *opacity = alpha;
                    }
                });
            }
            FormatMessage::FillOpacityInput(raw) => {
                self.format_ui.fill_opacity = raw.clone();
                if let Some(pct) = parse_f32(&raw) {
                    let alpha = pct_to_opacity(pct);
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(BoxFill::Solid { opacity, .. }) = &mut fmt.fill {
                            *opacity = alpha;
                        }
                    });
                }
            }
            FormatMessage::BorderMode(mode) => {
                self.commit_format_discrete(|fmt| {
                    fmt.border = match mode {
                        BorderMode::None => None,
                        BorderMode::Solid => Some(BoxBorder::default()),
                    };
                });
            }
            FormatMessage::BorderWidth(raw) => {
                self.format_ui.border_width = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let width = val.clamp(*BORDER_WIDTH.start(), *BORDER_WIDTH.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(border) = &mut fmt.border {
                            border.width = width;
                        }
                    });
                }
            }
            FormatMessage::BorderRadius(raw) => {
                self.format_ui.border_radius = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let radius = val.clamp(*CORNER_RADIUS.start(), *CORNER_RADIUS.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(border) = &mut fmt.border {
                            border.radius = radius;
                        }
                    });
                }
            }
            FormatMessage::BoxShadowMode(mode) => {
                self.commit_format_discrete(|fmt| {
                    fmt.shadow = match mode {
                        BoxShadowMode::None => None,
                        BoxShadowMode::Drop => Some(BoxShadow::default()),
                    };
                });
            }
            FormatMessage::BoxShadowOpacity(pct) => {
                let alpha = pct_to_opacity(pct);
                self.format_ui.box_shadow_opacity = pct_string(alpha);
                self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                    if let Some(shadow) = &mut fmt.shadow {
                        shadow.opacity = alpha;
                    }
                });
            }
            FormatMessage::BoxShadowOpacityInput(raw) => {
                self.format_ui.box_shadow_opacity = raw.clone();
                if let Some(pct) = parse_f32(&raw) {
                    let alpha = pct_to_opacity(pct);
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(shadow) = &mut fmt.shadow {
                            shadow.opacity = alpha;
                        }
                    });
                }
            }
            FormatMessage::BoxShadowBlur(raw) => {
                self.format_ui.box_shadow_blur = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let blur = val.clamp(*SHADOW_BLUR.start(), *SHADOW_BLUR.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(shadow) = &mut fmt.shadow {
                            shadow.blur = blur;
                        }
                    });
                }
            }
            FormatMessage::BoxShadowOffsetX(raw) => {
                self.format_ui.box_shadow_x = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let offset = val.clamp(*SHADOW_OFFSET.start(), *SHADOW_OFFSET.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(shadow) = &mut fmt.shadow {
                            shadow.offset_x = offset;
                        }
                    });
                }
            }
            FormatMessage::BoxShadowOffsetY(raw) => {
                self.format_ui.box_shadow_y = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let offset = val.clamp(*SHADOW_OFFSET.start(), *SHADOW_OFFSET.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(shadow) = &mut fmt.shadow {
                            shadow.offset_y = offset;
                        }
                    });
                }
            }
            FormatMessage::ElementOpacity(pct) => {
                let alpha = pct_to_opacity(pct);
                self.format_ui.element_opacity = pct_string(alpha);
                self.commit_format_grouped(EditGroup::FormatSlider, |fmt| fmt.opacity = alpha);
            }
            FormatMessage::ElementOpacityInput(raw) => {
                self.format_ui.element_opacity = raw.clone();
                if let Some(pct) = parse_f32(&raw) {
                    let alpha = pct_to_opacity(pct);
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| fmt.opacity = alpha);
                }
            }
            FormatMessage::ColorHexChanged(slot, raw) => {
                *hex_buffer_mut(&mut self.format_ui, slot) = raw.clone();
                if let Some(rgb) = Rgb::from_hex(raw.trim()) {
                    self.commit_format_grouped(EditGroup::FormatHex, |fmt| {
                        apply_slot_color(fmt, slot, rgb);
                    });
                }
            }
            FormatMessage::ColorPresetPicked(slot, rgb) => {
                self.commit_format_discrete(|fmt| apply_slot_color(fmt, slot, rgb));
            }
            FormatMessage::ResetTextColor => {
                self.commit_format_discrete(|fmt| fmt.text.color = None);
            }
            FormatMessage::FontFamilyPicked(family) => {
                self.format_ui.font_picker_open = false;
                self.commit_format_discrete(|fmt| fmt.text.font_family = family.map(Box::from));
            }
            FormatMessage::TextSizeMode(mode) => {
                self.commit_format_discrete(|fmt| {
                    fmt.text.font_size = match mode {
                        TextSizeMode::Theme => None,
                        TextSizeMode::AutoFit => Some(TextSize::AutoFit),
                        TextSizeMode::Fixed => Some(TextSize::Fixed(default_size)),
                    };
                });
            }
            FormatMessage::TextSizeInput(raw) => {
                self.format_ui.font_size = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let px = val.clamp(*size_range.start(), *size_range.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        fmt.text.font_size = Some(TextSize::Fixed(px));
                    });
                }
            }
            FormatMessage::TextSizeStep(delta) => {
                let box_rect = self.active_box_rect(&theme, target);
                let current_px = match box_format(&self.format, target).text.font_size {
                    Some(TextSize::Fixed(px)) => px,
                    _ => self
                        .deck
                        .slides
                        .get(self.selected)
                        .map(|s| {
                            slide::resolved_box_text_style(
                                s,
                                target == Target::Footer,
                                &theme,
                                box_rect,
                            )
                            .size
                        })
                        .unwrap_or(default_size),
                };
                let next = (current_px + delta).clamp(*size_range.start(), *size_range.end());
                self.commit_format_discrete(|fmt| {
                    fmt.text.font_size = Some(TextSize::Fixed(next));
                });
            }
            FormatMessage::ToggleBold => {
                let default_weight = match target {
                    Target::Body => theme.body_font.weight,
                    Target::Footer => theme.footer_font.weight,
                };
                self.commit_format_discrete(|fmt| {
                    let is_bold =
                        fmt.text.font_weight.unwrap_or(default_weight) == FontWeight::Bold;
                    fmt.text.font_weight = if is_bold {
                        if default_weight == FontWeight::Bold {
                            Some(FontWeight::Normal)
                        } else {
                            None
                        }
                    } else {
                        Some(FontWeight::Bold)
                    };
                });
            }
            FormatMessage::ToggleItalic => {
                self.commit_format_discrete(|fmt| fmt.text.italic = !fmt.text.italic);
            }
            FormatMessage::ToggleUnderline => {
                self.commit_format_discrete(|fmt| fmt.text.underline = !fmt.text.underline);
            }
            FormatMessage::StepIndent(delta) => {
                self.commit_format_discrete(|fmt| {
                    fmt.text.padding.left = (fmt.text.padding.left + delta)
                        .clamp(*BOX_PADDING.start(), *BOX_PADDING.end());
                });
            }
            FormatMessage::AlignX(ax) => {
                self.commit_format_discrete(|fmt| fmt.text.align_x = ax);
            }
            FormatMessage::AlignY(ay) => {
                self.commit_format_discrete(|fmt| fmt.text.align_y = ay);
            }
            FormatMessage::OutlineEnabled(enabled) => {
                self.commit_format_discrete(|fmt| {
                    fmt.text.outline = enabled.then(TextOutline::default);
                });
            }
            FormatMessage::OutlineWidth(raw) => {
                self.format_ui.outline_width = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let width = val.clamp(*OUTLINE_WIDTH.start(), *OUTLINE_WIDTH.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(outline) = &mut fmt.text.outline {
                            outline.width = width;
                        }
                    });
                }
            }
            FormatMessage::TextShadowMode(mode) => {
                self.commit_format_discrete(|fmt| {
                    fmt.text.shadow = match mode {
                        TextShadowMode::Theme => None,
                        TextShadowMode::None => Some(TextShadowOverride::None),
                        TextShadowMode::Drop => {
                            Some(TextShadowOverride::Drop(TextShadow::default()))
                        }
                    };
                });
            }
            FormatMessage::TextShadowOpacity(pct) => {
                let alpha = pct_to_opacity(pct);
                self.format_ui.text_shadow_opacity = pct_string(alpha);
                self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                    if let Some(TextShadowOverride::Drop(shadow)) = &mut fmt.text.shadow {
                        shadow.opacity = alpha;
                    }
                });
            }
            FormatMessage::TextShadowOpacityInput(raw) => {
                self.format_ui.text_shadow_opacity = raw.clone();
                if let Some(pct) = parse_f32(&raw) {
                    let alpha = pct_to_opacity(pct);
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(TextShadowOverride::Drop(shadow)) = &mut fmt.text.shadow {
                            shadow.opacity = alpha;
                        }
                    });
                }
            }
            FormatMessage::TextShadowOffsetX(raw) => {
                self.format_ui.text_shadow_x = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let offset = val.clamp(*SHADOW_OFFSET.start(), *SHADOW_OFFSET.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(TextShadowOverride::Drop(shadow)) = &mut fmt.text.shadow {
                            shadow.offset_x = offset;
                        }
                    });
                }
            }
            FormatMessage::TextShadowOffsetY(raw) => {
                self.format_ui.text_shadow_y = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let offset = val.clamp(*SHADOW_OFFSET.start(), *SHADOW_OFFSET.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        if let Some(TextShadowOverride::Drop(shadow)) = &mut fmt.text.shadow {
                            shadow.offset_y = offset;
                        }
                    });
                }
            }
            FormatMessage::BulletStyle(style) => {
                self.commit_format_discrete(|fmt| fmt.text.bullets = style);
            }
            FormatMessage::ToggleLineHeightFixed(fixed) => {
                self.commit_format_discrete(|fmt| {
                    fmt.text.line_height =
                        fixed.then(|| fmt.text.line_height.unwrap_or(default_lh));
                });
            }
            FormatMessage::LineHeightChanged(lh) => {
                let clamped = lh.clamp(*LINE_HEIGHT.start(), *LINE_HEIGHT.end());
                self.format_ui.line_height = format!("{clamped:.2}");
                self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                    fmt.text.line_height = Some(clamped);
                });
            }
            FormatMessage::LineHeightInput(raw) => {
                self.format_ui.line_height = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let clamped = val.clamp(*LINE_HEIGHT.start(), *LINE_HEIGHT.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| {
                        fmt.text.line_height = Some(clamped);
                    });
                }
            }
            FormatMessage::PaddingChanged(side, raw) => {
                *match side {
                    PaddingSide::Left => &mut self.format_ui.pad_left,
                    PaddingSide::Right => &mut self.format_ui.pad_right,
                    PaddingSide::Top => &mut self.format_ui.pad_top,
                    PaddingSide::Bottom => &mut self.format_ui.pad_bottom,
                } = raw.clone();
                if let Some(val) = parse_f32(&raw) {
                    let px = val.clamp(*BOX_PADDING.start(), *BOX_PADDING.end());
                    self.commit_format_grouped(EditGroup::FormatSlider, |fmt| match side {
                        PaddingSide::Left => fmt.text.padding.left = px,
                        PaddingSide::Right => fmt.text.padding.right = px,
                        PaddingSide::Top => fmt.text.padding.top = px,
                        PaddingSide::Bottom => fmt.text.padding.bottom = px,
                    });
                }
            }
            FormatMessage::Order(action) => {
                let footer_on_top = match (target, action) {
                    (Target::Body, OrderAction::Front | OrderAction::Forward)
                    | (Target::Footer, OrderAction::Back | OrderAction::Backward) => false,
                    (Target::Footer, OrderAction::Front | OrderAction::Forward)
                    | (Target::Body, OrderAction::Back | OrderAction::Backward) => true,
                };
                if self.format.footer_on_top != footer_on_top {
                    self.push_history();
                    self.format.footer_on_top = footer_on_top;
                    self.notice = None;
                    self.recompile();
                }
            }
            FormatMessage::Align(align) => {
                if box_format(&self.format, target).locked || self.selected >= self.slides.len() {
                    return;
                }
                let next = align_box_on_canvas(
                    self.active_box_rect(&theme, target),
                    &theme,
                    target,
                    align,
                );
                self.push_history();
                self.set_selected_box_rect(target, next);
                self.sync_format_inputs();
                self.recompile();
            }
            FormatMessage::DimChanged(field, raw) => {
                *match field {
                    DimField::Left => &mut self.format_ui.dim_left,
                    DimField::Top => &mut self.format_ui.dim_top,
                    DimField::Width => &mut self.format_ui.dim_width,
                    DimField::Height => &mut self.format_ui.dim_height,
                } = raw.clone();
                if box_format(&self.format, target).locked || self.selected >= self.slides.len() {
                    return;
                }
                let Some(val) = parse_f32(&raw) else {
                    return;
                };
                let canvas_h = theme.layout.height();
                let current = self.active_box_rect(&theme, target).clamped();
                let prev_w_px = (current.width * CANVAS_WIDTH).max(1.0);
                let prev_h_px = (current.height * canvas_h).max(1.0);
                let mut next = current;
                match field {
                    DimField::Left => {
                        next.x = (val / CANVAS_WIDTH).clamp(0.0, 1.0 - current.width);
                    }
                    DimField::Top => {
                        next.y = (val / canvas_h).clamp(0.0, 1.0 - current.height);
                    }
                    DimField::Width => {
                        let w_norm = (val / CANVAS_WIDTH).clamp(MIN_BOX_SIZE, 1.0 - next.x);
                        next.width = w_norm;
                        if self.format_ui.constrain_proportions && prev_w_px > 0.0 {
                            let h_px = (w_norm * CANVAS_WIDTH) * (prev_h_px / prev_w_px);
                            next.height = (h_px / canvas_h).clamp(MIN_BOX_SIZE, 1.0 - next.y);
                            self.format_ui.dim_height = round_string(next.height * canvas_h);
                        }
                    }
                    DimField::Height => {
                        let h_norm = (val / canvas_h).clamp(MIN_BOX_SIZE, 1.0 - next.y);
                        next.height = h_norm;
                        if self.format_ui.constrain_proportions && prev_h_px > 0.0 {
                            let w_px = (h_norm * canvas_h) * (prev_w_px / prev_h_px);
                            next.width = (w_px / CANVAS_WIDTH).clamp(MIN_BOX_SIZE, 1.0 - next.x);
                            self.format_ui.dim_width = round_string(next.width * CANVAS_WIDTH);
                        }
                    }
                }
                let next = next.clamped();
                self.history
                    .begin_group(EditGroup::FormatDimension, self.snapshot());
                self.set_selected_box_rect(target, next);
                self.recompile();
            }
            FormatMessage::SetLocked(locked) => {
                if box_format(&self.format, target).locked != locked {
                    self.commit_format_discrete(|fmt| fmt.locked = locked);
                }
            }
        }
    }

    fn commit_format_discrete(&mut self, mutate: impl FnOnce(&mut BoxFormat)) {
        let target = self.format_target();
        self.push_history();
        mutate(box_format_mut(&mut self.format, target));
        self.notice = None;
        self.sync_format_inputs();
        self.recompile();
    }

    fn commit_format_grouped(&mut self, group: EditGroup, mutate: impl FnOnce(&mut BoxFormat)) {
        let target = self.format_target();
        let before = self.snapshot();
        self.history.begin_group(group, before);
        mutate(box_format_mut(&mut self.format, target));
        self.notice = None;
        self.recompile();
    }

    fn set_selected_box_rect(&mut self, target: Target, rect: SlideBox) {
        if let Some(slide) = self.slides.get_mut(self.selected) {
            match target {
                Target::Body => slide.body_box = Some(rect.clamped()),
                Target::Footer => slide.footer_box = Some(rect.clamped()),
            }
            self.customized = true;
            self.notice = None;
        }
    }
}

fn font_spec(family: &str) -> FontSpec {
    FontSpec {
        family: family.into(),
        weight: FontWeight::Normal,
    }
}

fn parse_f32(raw: &str) -> Option<f32> {
    let value: f32 = raw.trim().parse().ok()?;
    value.is_finite().then_some(value)
}

fn pct_to_opacity(pct: f32) -> f32 {
    (pct / 100.0).clamp(*OPACITY.start(), *OPACITY.end())
}

fn hex_buffer_mut(ui: &mut FormatState, slot: ColorSlot) -> &mut String {
    match slot {
        ColorSlot::Fill => &mut ui.fill_hex,
        ColorSlot::Border => &mut ui.border_hex,
        ColorSlot::BoxShadow => &mut ui.box_shadow_hex,
        ColorSlot::Text => &mut ui.text_hex,
        ColorSlot::Outline => &mut ui.outline_hex,
        ColorSlot::TextShadow => &mut ui.text_shadow_hex,
    }
}

fn apply_slot_color(fmt: &mut BoxFormat, slot: ColorSlot, rgb: Rgb) {
    let rgb = rgb.clamped();
    match slot {
        ColorSlot::Fill => {
            if let Some(BoxFill::Solid { color, .. }) = &mut fmt.fill {
                *color = rgb;
            }
        }
        ColorSlot::Border => {
            if let Some(border) = &mut fmt.border {
                border.color = rgb;
            }
        }
        ColorSlot::BoxShadow => {
            if let Some(shadow) = &mut fmt.shadow {
                shadow.color = rgb;
            }
        }
        ColorSlot::Text => {
            fmt.text.color = Some(rgb);
        }
        ColorSlot::Outline => {
            if let Some(outline) = &mut fmt.text.outline {
                outline.color = rgb;
            }
        }
        ColorSlot::TextShadow => {
            if let Some(TextShadowOverride::Drop(shadow)) = &mut fmt.text.shadow {
                shadow.color = rgb;
            }
        }
    }
}

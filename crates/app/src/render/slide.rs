use iced::widget::{center, column, container, responsive, text};
use iced::{Color, Element, Fill};
use sdw_core::slide::Slide;
use sdw_core::theme::{CANVAS_RATIO, CANVAS_WIDTH, Rgb, SlideTheme, TextAlign};

pub fn color(rgb: Rgb) -> Color {
    Color::from_rgb(rgb.r, rgb.g, rgb.b)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visibility {
    pub background: bool,
    pub body: bool,
}

impl Visibility {
    pub const FULL: Self = Self {
        background: true,
        body: true,
    };
}

// Kotak 16:9 yang mengisi ruang yang tersedia tanpa pernah mengubah proporsi. Dipakai
// preview operator, panel live, dan jendela proyektor — satu fungsi, sehingga apa yang
// dilihat operator selalu sama dengan yang dilihat jemaat.
pub fn stage<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
) -> Element<'a, Message>
where
    Message: 'a,
{
    responsive(move |size| {
        let width = size.width.min(size.height * CANVAS_RATIO);

        center(canvas(slide, theme, visibility, width)).into()
    })
    .into()
}

pub fn thumbnail<'a, Message>(
    slide: &'a Slide,
    theme: &'a SlideTheme,
    width: f32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    canvas(Some(slide), theme, Visibility::FULL, width)
}

fn canvas<'a, Message>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
    visibility: Visibility,
    width: f32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    // Semua ukuran tema dinyatakan pada kanvas 1920x1080, lalu diskalakan sekali di
    // sini. Itu sebabnya thumbnail 160 px dan proyektor 1920 px punya tata letak sama.
    let scale = width / CANVAS_WIDTH;
    let height = width / CANVAS_RATIO;

    let background = if visibility.background {
        color(theme.background)
    } else {
        Color::BLACK
    };

    let body = slide.filter(|_| visibility.body).map(|slide| {
        let mut lines = column![
            text(&*slide.body)
                .size(theme.body_size * scale)
                .line_height(text::LineHeight::Relative(theme.line_height))
                .color(color(theme.body))
                .align_x(alignment(theme.align))
                .width(Fill),
        ]
        .spacing(theme.footer_size * scale)
        .width(Fill);

        if let Some(footer) = &slide.footer {
            lines = lines.push(
                text(&**footer)
                    .size(theme.footer_size * scale)
                    .color(color(theme.footer))
                    .align_x(alignment(theme.align))
                    .width(Fill),
            );
        }

        lines
    });

    container(container(body).center(Fill).padding(theme.margin * scale))
        .width(width)
        .height(height)
        .style(move |_theme| container::Style {
            background: Some(background.into()),
            ..container::Style::default()
        })
        .into()
}

fn alignment(align: TextAlign) -> text::Alignment {
    match align {
        TextAlign::Left => text::Alignment::Left,
        TextAlign::Center => text::Alignment::Center,
        TextAlign::Right => text::Alignment::Right,
    }
}

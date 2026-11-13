use iced::widget::{Column, Space, container, row, text};
use iced::window::icon::{self, Icon};
use iced::{Center, Color, Element};

use crate::style;

pub struct Cut {
    rows: &'static [f32],
    row: f32,
    gap: f32,
}

pub const SMALL: Cut = Cut {
    rows: &[64.0, 144.0, 64.0, 64.0, 64.0],
    row: 24.0,
    gap: 16.0,
};

impl Cut {
    fn height(&self) -> f32 {
        self.rows.len() as f32 * (self.row + self.gap) - self.gap
    }
}

pub fn mark<'a, Message: 'a>(cut: &Cut, height: f32) -> Element<'a, Message> {
    mark_with_color(cut, height, None)
}

pub fn mark_colored<'a, Message: 'a>(cut: &Cut, height: f32, color: Color) -> Element<'a, Message> {
    mark_with_color(cut, height, Some(color))
}

fn mark_with_color<'a, Message: 'a>(
    cut: &Cut,
    height: f32,
    color: Option<Color>,
) -> Element<'a, Message> {
    let scale = height / cut.height();
    let row = cut.row * scale;

    Column::with_children(cut.rows.iter().map(|width| {
        let cell = container(Space::new()).width(width * scale).height(row);
        match color {
            Some(color) => cell.style(style::rounded(color, row / 2.0)).into(),
            None => cell.style(style::logo_row(row / 2.0)).into(),
        }
    }))
    .spacing(cut.gap * scale)
    .align_x(Center)
    .into()
}

pub fn wordmark<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    wordmark_colored(size, style::TEXT, style::TEXT_2)
}

pub fn wordmark_colored<'a, Message: 'a>(
    size: f32,
    primary: Color,
    secondary: Color,
) -> Element<'a, Message> {
    row![
        text("Solideo").size(size).font(style::BOLD).color(primary),
        text("Worship")
            .size(size)
            .font(style::INTER)
            .color(secondary),
    ]
    .into()
}

const ICON_SIZE: u32 = 32;
const ICON_RADIUS: f32 = 6.0;
const ICON_TILE: Color = Color::from_rgb8(0x1C, 0x1D, 0x21);
const ICON_MARK: Color = Color::from_rgb8(0xF2, 0xF2, 0xF3);
const ICON_CROSS: [(u32, u32, u32, u32); 2] = [(14, 4, 4, 24), (8, 10, 16, 4)];

pub fn window_icon() -> Option<Icon> {
    let mut rgba = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);

    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let on_mark = ICON_CROSS.iter().any(|&(left, top, width, height)| {
                (left..left + width).contains(&x) && (top..top + height).contains(&y)
            });
            let [red, green, blue, _] = if on_mark { ICON_MARK } else { ICON_TILE }.into_rgba8();
            rgba.extend([red, green, blue, tile_alpha(x, y)]);
        }
    }

    icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).ok()
}

fn tile_alpha(x: u32, y: u32) -> u8 {
    let size = ICON_SIZE as f32;
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    let corner_x = px.clamp(ICON_RADIUS, size - ICON_RADIUS);
    let corner_y = py.clamp(ICON_RADIUS, size - ICON_RADIUS);
    let distance = (px - corner_x).hypot(py - corner_y);
    let coverage = (ICON_RADIUS - distance + 0.5).clamp(0.0, 1.0);

    (coverage * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
        let start = ((y * ICON_SIZE + x) * 4) as usize;
        rgba[start..start + 4].try_into().unwrap()
    }

    fn mark_rects(svg: &str) -> Vec<[f32; 4]> {
        svg.split("<rect")
            .skip(1)
            .map(|tag| {
                let tag = &tag[..tag.find('>').unwrap()];
                ["x", "y", "width", "height"].map(|name| {
                    tag.split_once(&format!(" {name}=\""))
                        .map_or(0.0, |(_, rest)| {
                            rest[..rest.find('"').unwrap()].parse().unwrap()
                        })
                })
            })
            .filter(|&[.., width, height]| width < 256.0 && height < 256.0)
            .collect()
    }

    #[test]
    fn cuts_match_the_svg_masters() {
        let masters = [(include_str!("../../../assets/logo/app-icon-32.svg"), &SMALL)];

        for (svg, cut) in masters {
            let rects = mark_rects(svg);
            let widths: Vec<f32> = rects.iter().map(|&[_, _, width, _]| width).collect();

            assert_eq!(widths, cut.rows);
            assert!(rects.iter().all(|&[.., height]| height == cut.row));
            assert!(
                rects
                    .windows(2)
                    .all(|pair| pair[1][1] - pair[0][1] == cut.row + cut.gap)
            );
        }
    }

    #[test]
    fn window_icon_cross_matches_the_16_px_master() {
        let cross: Vec<_> = mark_rects(include_str!("../../../assets/logo/app-icon-16.svg"))
            .into_iter()
            .map(|rect| rect.map(|value| value as u32 * ICON_SIZE / 256))
            .map(|[x, y, width, height]| (x, y, width, height))
            .collect();

        assert_eq!(cross, ICON_CROSS);
    }

    #[test]
    fn window_icon_draws_the_cross_on_a_rounded_tile() {
        let (rgba, size) = window_icon().unwrap().into_raw();

        assert_eq!((size.width, size.height), (ICON_SIZE, ICON_SIZE));
        assert_eq!(pixel(&rgba, 0, 0)[3], 0);
        assert_eq!(pixel(&rgba, 31, 31)[3], 0);
        assert_eq!(pixel(&rgba, 2, 16), [0x1C, 0x1D, 0x21, 255]);
        assert_eq!(pixel(&rgba, 16, 12), [0xF2, 0xF2, 0xF3, 255]);
        assert_eq!(pixel(&rgba, 16, 27), [0xF2, 0xF2, 0xF3, 255]);
        assert_eq!(pixel(&rgba, 16, 28), [0x1C, 0x1D, 0x21, 255]);
    }
}

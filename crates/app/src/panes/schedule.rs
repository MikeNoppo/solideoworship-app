use iced::widget::{Column, Space, column, container, mouse_area, opaque, row, space, stack, text};
use iced::{Bottom, Center, Element, Fill, Padding, Point, Top};
use sdw_core::{BackgroundMedia, ContentKind};

use crate::i18n::t;
use crate::icon;
use crate::message::{MediaDrop, Message};
use crate::panes::{live_badge, media};
use crate::popover::{Align, popover, right_click};
use crate::schedule::Entry;
use crate::style;
use crate::ui::{self, label};
use crate::{App, Drag, ScheduleMenu};

const ROW_HEIGHT: f32 = 52.0;
const ROW_GAP: f32 = 4.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let song_drop = match &app.drag {
        Some(Drag::Song { target, .. }) => Some(*target),
        Some(Drag::Media { target, .. }) => Some(target.and_then(MediaDrop::insertion)),
        _ => None,
    };
    let background_target = match &app.drag {
        Some(Drag::Media { target, .. }) => target.and_then(MediaDrop::background),
        _ => None,
    };

    let header = row![
        label(t().schedule.title, 13.0, style::SEMIBOLD, style::TEXT),
        space::horizontal(),
        label(
            &app.captions.schedule_count,
            12.0,
            style::INTER,
            style::TEXT_3
        ),
    ]
    .spacing(8)
    .padding(Padding {
        top: 0.0,
        right: 10.0,
        bottom: 10.0,
        left: 10.0,
    })
    .align_y(Center);

    let body: Element<'_, Message> = if app.schedule.is_empty() {
        let is_drop_target = song_drop == Some(Some(0));

        container(
            container(ui::empty(
                icon::LIST_PLUS,
                true,
                t().schedule.empty_title,
                Some(t().schedule.empty_body),
            ))
            .width(Fill)
            .padding([24, 16])
            .style(style::boxed(
                is_drop_target.then_some(style::SURFACE),
                if is_drop_target {
                    style::INVERSE
                } else {
                    iced::Color::TRANSPARENT
                },
                10.0,
            )),
        )
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 80.0,
            left: 12.0,
        })
        .center(Fill)
        .into()
    } else {
        let len = app.schedule.entries().len();
        let last = len - 1;
        let drop_target = song_drop.flatten();
        let rows = app.schedule.entries().iter().enumerate().fold(
            Column::new().spacing(ROW_GAP),
            |list, (index, entry)| {
                list.push(item(
                    app,
                    index,
                    entry,
                    last,
                    drop_target,
                    background_target,
                ))
            },
        );

        let rows = mouse_area(rows).on_exit(match &app.drag {
            Some(Drag::Song { .. }) => Message::SongDropTarget(Some(len)),
            Some(Drag::Media { .. }) => Message::MediaDropTarget(Some(MediaDrop::Insert(len))),
            _ => Message::RowHovered(None),
        });
        let rows = match &app.drag {
            Some(Drag::Row { .. }) => rows.on_move(move |position| {
                let row = (position.y / (ROW_HEIGHT + ROW_GAP)).max(0.0) as usize;
                Message::DragOver(row.min(last))
            }),
            Some(Drag::Song { .. }) => rows.on_move(move |position| {
                Message::SongDropTarget(Some(insertion_index(position.y, len)))
            }),
            Some(Drag::Media { kind, .. }) => {
                let kind = *kind;
                rows.on_move(move |position| {
                    Message::MediaDropTarget(Some(media_drop(position.y, len, |index| {
                        app.schedule.takes_background(index, kind)
                    })))
                })
            }
            None => rows,
        };

        let tip = (app.schedule.entries().len() <= 2).then(|| {
            container(
                column![
                    label(t().schedule.tip_title, 12.0, style::SEMIBOLD, style::TEXT),
                    label(t().schedule.tip_body, 12.0, style::INTER, style::TEXT_3)
                        .line_height(1.45),
                ]
                .spacing(6),
            )
            .padding(12)
            .width(Fill)
            .style(style::boxed(Some(style::SURFACE), style::BORDER, 8.0))
        });

        column![ui::scroll(rows)].push(tip).spacing(4).into()
    };

    let pane = right_click(
        container(column![header, body].spacing(4))
            .width(app.columns().schedule)
            .height(Fill)
            .padding([16, 12]),
        |position| Message::ScheduleContextMenu(None, position),
    );
    let pane = mouse_area(pane);
    let pane = if song_drop.is_some() {
        pane.on_enter(Message::DragEnteredSchedule)
            .on_exit(Message::DragExitedSchedule)
            .interaction(iced::mouse::Interaction::Grabbing)
    } else {
        pane
    };

    let (menu_position, menu_popup) = match app.schedule_menu {
        Some(menu) => (menu.position, Some(context_menu(app, menu))),
        None => (Point::ORIGIN, None),
    };

    popover(
        pane,
        menu_popup,
        Align::At(menu_position),
        Message::MenuDismissed,
    )
    .into()
}

fn context_menu(app: &App, menu: ScheduleMenu) -> Element<'static, Message> {
    let entries = app.schedule.entries();
    let total = entries.len();
    let target = menu.target.filter(|&index| index < total);
    let song_id = target.and_then(|index| entries[index].origin());

    let add_row = mouse_area(ui::menu_item(
        Some(icon::PLUS),
        t().schedule.add_item,
        Some(icon::view(icon::ARROW_RIGHT, 12.0, style::TEXT_3).into()),
        Some(Message::ScheduleAddSubmenu(!menu.add_open)),
    ))
    .on_enter(Message::ScheduleAddSubmenu(true));

    let clear_background = target
        .filter(|&index| entries[index].background().is_some())
        .map(|index| {
            ui::menu_item(
                Some(icon::ERASER),
                t().schedule.clear_background,
                None,
                Some(Message::ScheduleBackgroundCleared(index)),
            )
        });

    let actions = mouse_area(
        Column::new()
            .push(ui::menu_item(
                Some(icon::PLAY),
                t().toolbar.go_live,
                None,
                target.map(|_| Message::GoLive),
            ))
            .push(ui::menu_item(
                Some(icon::PENCIL),
                t().slides.edit_song,
                None,
                song_id.map(Message::SongEditRequested),
            ))
            .push(clear_background)
            .push(separator())
            .push(ui::menu_item(
                None,
                t().schedule.move_up,
                None,
                target
                    .filter(|&index| index > 0)
                    .map(|index| Message::ScheduleMoved(index, index - 1)),
            ))
            .push(ui::menu_item(
                None,
                t().schedule.move_down,
                None,
                target
                    .filter(|&index| index + 1 < total)
                    .map(|index| Message::ScheduleMoved(index, index + 1)),
            ))
            .push(separator())
            .push(ui::menu_item(
                None,
                t().schedule.duplicate,
                None,
                target.map(Message::ScheduleDuplicated),
            ))
            .push(ui::menu_item(
                Some(icon::TRASH_2),
                t().schedule.remove,
                None,
                target.map(Message::ScheduleRemoved),
            ))
            .spacing(2),
    )
    .on_enter(Message::ScheduleAddSubmenu(false));

    let main: Element<'static, Message> =
        container(column![add_row, separator(), actions].spacing(2))
            .width(220)
            .padding(6)
            .style(style::menu)
            .into();

    if !menu.add_open {
        return main;
    }

    let submenu: Element<'static, Message> = container(
        column![
            ui::menu_item(
                Some(icon::MUSIC),
                t().common.new_song,
                None,
                Some(Message::SongCreateRequested),
            ),
            ui::menu_item(
                Some(icon::BOOK_OPEN),
                t().schedule.add_scripture,
                None,
                Some(Message::ScheduleScriptureRequested),
            ),
            separator(),
            ui::menu_item(
                Some(icon::FOLDER_OPEN),
                t().schedule.open_schedule,
                None,
                Some(Message::ScheduleOpenRequested),
            ),
        ]
        .spacing(2),
    )
    .width(210)
    .padding(6)
    .style(style::menu)
    .into();

    mouse_area(row![opaque(main), opaque(submenu)].spacing(4).align_y(Top))
        .on_press(Message::MenuDismissed)
        .on_right_press(Message::MenuDismissed)
        .into()
}

fn separator() -> Element<'static, Message> {
    container(ui::hrule(style::BORDER)).padding([4, 0]).into()
}

fn insertion_index(y: f32, len: usize) -> usize {
    if y <= 0.0 || len == 0 {
        return 0;
    }

    let step = ROW_HEIGHT + ROW_GAP;
    let slot = (y / step) as usize;
    let within = y - slot as f32 * step;
    let index = if within >= ROW_HEIGHT * 0.5 {
        slot + 1
    } else {
        slot
    };

    index.min(len)
}

fn media_drop(y: f32, len: usize, takes_background: impl Fn(usize) -> bool) -> MediaDrop {
    let step = ROW_HEIGHT + ROW_GAP;
    if y >= 0.0 && len > 0 {
        let slot = (y / step) as usize;
        let within = y - slot as f32 * step;
        let middle = ROW_HEIGHT * 0.25..ROW_HEIGHT * 0.75;
        if slot < len && middle.contains(&within) && takes_background(slot) {
            return MediaDrop::Background(slot);
        }
    }

    MediaDrop::Insert(insertion_index(y, len))
}

fn background_chip(background: &BackgroundMedia) -> Element<'_, Message> {
    let media = background.media();
    container(
        row![
            icon::view(media::glyph(media.kind), 11.0, style::TEXT_3),
            label(&media.title, 11.0, style::INTER, style::TEXT_3).wrapping(text::Wrapping::None),
        ]
        .spacing(4)
        .align_y(Center),
    )
    .padding([1, 6])
    .max_width(160)
    .clip(true)
    .style(style::boxed(Some(style::RAISED), style::BORDER, 4.0))
    .into()
}

fn drop_line<'a>() -> Element<'a, Message> {
    row![
        ui::dot(6.0, style::INVERSE),
        container(Space::new())
            .width(Fill)
            .height(2)
            .style(style::rounded(style::INVERSE, 1.0)),
    ]
    .align_y(Center)
    .into()
}

fn item<'a>(
    app: &'a App,
    index: usize,
    entry: &'a Entry,
    last: usize,
    drop_target: Option<usize>,
    background_target: Option<usize>,
) -> Element<'a, Message> {
    let is_selected = app.schedule.selected() == Some(index);
    let is_live = app.output.is_some() && app.is_live(entry);

    let glyph = match entry.deck().kind {
        ContentKind::Song => icon::MUSIC,
        ContentKind::Scripture => icon::BOOK_OPEN,
        ContentKind::Media => match entry.item() {
            sdw_core::ScheduleItem::Media(media) => match media.kind {
                sdw_core::MediaKind::Image => icon::LAYOUT_GRID,
                sdw_core::MediaKind::Video => icon::PLAY,
                sdw_core::MediaKind::Audio => icon::MUSIC,
            },
            _ => icon::LAYOUT_GRID,
        },
    };

    let is_target = matches!(
        app.drag,
        Some(Drag::Row { from, over }) if over == index && from != index
    );
    let is_background_target = background_target == Some(index);

    let content = row![
        label(ui::number(index + 1), 12.0, style::MEDIUM, style::TEXT_3),
        container(icon::view(glyph, 14.0, style::TEXT_2))
            .center(28)
            .style(style::boxed(Some(style::RAISED), style::BORDER, 6.0)),
        column![
            label(&entry.deck().title, 13.0, style::MEDIUM, style::TEXT),
            row![label(entry.meta(), 12.0, style::INTER, style::TEXT_3)]
                .push(entry.background().map(background_chip))
                .spacing(6)
                .align_y(Center),
        ]
        .spacing(2)
        .width(Fill),
    ]
    .push(is_live.then(live_badge))
    .spacing(10)
    .height(Fill)
    .align_y(Center);

    let background = if is_selected || is_background_target {
        Some(style::ACTIVE)
    } else if app.hovered_row == Some(index) && app.drag.is_none() {
        Some(style::RAISED)
    } else {
        None
    };

    let area = mouse_area(
        container(content)
            .width(Fill)
            .height(ROW_HEIGHT)
            .padding([0, 10])
            .style(style::boxed(
                background,
                if is_background_target {
                    style::INVERSE
                } else if is_target {
                    style::TEXT_3
                } else {
                    iced::Color::TRANSPARENT
                },
                8.0,
            )),
    )
    .interaction(if app.drag.is_some() {
        iced::mouse::Interaction::Grabbing
    } else {
        iced::mouse::Interaction::Pointer
    });

    let area = match &app.drag {
        None => area
            .on_press(Message::DragStarted(index))
            .on_enter(Message::RowHovered(Some(index))),
        Some(Drag::Row { .. }) => area.on_release(Message::DragEnded),
        Some(Drag::Song { .. } | Drag::Media { .. }) => area,
    };

    let row_box = right_click(area, move |position| {
        Message::ScheduleContextMenu(Some(index), position)
    });

    if drop_target == Some(index) {
        stack![
            row_box,
            container(drop_line()).width(Fill).height(Fill).align_y(Top),
        ]
        .into()
    } else if index == last && drop_target == Some(last + 1) {
        stack![
            row_box,
            container(drop_line())
                .width(Fill)
                .height(Fill)
                .align_y(Bottom),
        ]
        .into()
    } else {
        row_box.into()
    }
}

#[cfg(test)]
mod tests {
    use super::{insertion_index, media_drop};
    use crate::message::MediaDrop;

    #[test]
    fn media_drop_sets_backgrounds_in_the_middle_and_inserts_at_the_edges() {
        let every = |_| true;
        assert_eq!(media_drop(-5.0, 3, every), MediaDrop::Insert(0));
        assert_eq!(media_drop(5.0, 3, every), MediaDrop::Insert(0));
        assert_eq!(media_drop(13.0, 3, every), MediaDrop::Background(0));
        assert_eq!(media_drop(38.0, 3, every), MediaDrop::Background(0));
        assert_eq!(media_drop(40.0, 3, every), MediaDrop::Insert(1));
        assert_eq!(media_drop(54.0, 3, every), MediaDrop::Insert(1));
        assert_eq!(media_drop(82.0, 3, every), MediaDrop::Background(1));
        assert_eq!(media_drop(500.0, 3, every), MediaDrop::Insert(3));
        assert_eq!(media_drop(20.0, 0, every), MediaDrop::Insert(0));
    }

    #[test]
    fn media_drop_only_inserts_on_rows_without_backgrounds() {
        let songs_only = |index| index != 1;
        assert_eq!(media_drop(82.0, 3, songs_only), MediaDrop::Insert(2));
        assert_eq!(media_drop(70.0, 3, songs_only), MediaDrop::Insert(1));
        assert_eq!(media_drop(90.0, 3, songs_only), MediaDrop::Insert(2));
        assert_eq!(media_drop(140.0, 3, songs_only), MediaDrop::Background(2));
        assert_eq!(media_drop(26.0, 3, |_| false), MediaDrop::Insert(1));
    }

    #[test]
    fn insertion_index_splits_rows_and_gaps_smoothly() {
        assert_eq!(insertion_index(-10.0, 3), 0);
        assert_eq!(insertion_index(0.0, 3), 0);
        assert_eq!(insertion_index(25.0, 3), 0);
        assert_eq!(insertion_index(26.0, 3), 1);
        assert_eq!(insertion_index(54.0, 3), 1);
        assert_eq!(insertion_index(56.0, 3), 1);
        assert_eq!(insertion_index(82.0, 3), 2);
        assert_eq!(insertion_index(500.0, 3), 3);
        assert_eq!(insertion_index(10.0, 0), 0);
    }
}

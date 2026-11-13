use super::*;
use iced::Length::FillPortion;
use iced::widget::column;
use sdw_core::settings::SettingsError;

#[derive(Debug, Clone, Copy)]
pub enum LabelField {
    Name,
    Aliases,
    Color,
    Shortcut,
}

impl LabelField {
    pub(super) fn set(self, draft: &mut LabelDraft, value: String) {
        let field = match self {
            Self::Name => &mut draft.name,
            Self::Aliases => &mut draft.aliases,
            Self::Color => &mut draft.color,
            Self::Shortcut => &mut draft.shortcut,
        };
        *field = value;
        draft.parsed = draft.rule().ok();
    }
}

pub(super) struct LabelDraft {
    name: String,
    aliases: String,
    color: String,
    shortcut: String,
    parsed: Option<LabelRule>,
}

impl LabelDraft {
    pub(super) fn empty() -> Self {
        let mut draft = Self {
            name: String::new(),
            aliases: String::new(),
            color: "#305384".into(),
            shortcut: String::new(),
            parsed: None,
        };
        draft.parsed = draft.rule().ok();
        draft
    }

    fn rule(&self) -> Result<LabelRule, SettingsError> {
        let color = Rgb::from_hex(&self.color).ok_or(SettingsError::InvalidLabels)?;
        let shortcut = if self.shortcut.is_empty() {
            None
        } else {
            let mut letters = self.shortcut.chars();
            match (letters.next(), letters.next()) {
                (Some(letter), None) if letter.is_ascii_uppercase() => Some(letter),
                _ => return Err(SettingsError::InvalidLabels),
            }
        };
        Ok(LabelRule {
            name: self.name.trim().into(),
            aliases: if self.aliases.trim().is_empty() {
                Vec::new()
            } else {
                self.aliases
                    .split(',')
                    .map(|name| name.trim().into())
                    .collect()
            },
            color,
            shortcut,
        })
    }
}

impl ThemeDialog {
    pub(super) fn sync_labels(&mut self) {
        self.labels = self
            .draft
            .labels
            .rules
            .iter()
            .map(|rule| LabelDraft {
                name: rule.name.to_string(),
                aliases: rule
                    .aliases
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<&str>>()
                    .join(", "),
                color: rule.color.to_hex(),
                shortcut: rule
                    .shortcut
                    .map_or_else(String::new, |letter| letter.to_string()),
                parsed: Some(rule.clone()),
            })
            .collect();
    }

    pub(super) fn validated_labels(&self) -> Result<LabelSettings, SettingsError> {
        let labels = LabelSettings {
            rules: self
                .labels
                .iter()
                .map(|draft| draft.parsed.clone().ok_or(SettingsError::InvalidLabels))
                .collect::<Result<_, _>>()?,
        };
        labels.validate()?;
        Ok(labels)
    }

    pub(super) fn update_label_preview(&mut self) {
        if let Ok(labels) = self.validated_labels() {
            self.draft.labels = labels.clone();
            self.preview_settings.labels = labels;
        }
    }

    pub(super) fn labels_form(&self) -> Element<'_, Message> {
        let header = row![
            label(
                t().settings.label_name,
                11.0,
                style::SEMIBOLD,
                style::TEXT_3
            )
            .width(FillPortion(2)),
            label(
                t().settings.label_aliases,
                11.0,
                style::SEMIBOLD,
                style::TEXT_3
            )
            .width(FillPortion(3)),
            label(
                t().settings.label_color,
                11.0,
                style::SEMIBOLD,
                style::TEXT_3
            )
            .width(112),
            label(
                t().settings.label_shortcut,
                11.0,
                style::SEMIBOLD,
                style::TEXT_3
            )
            .width(70),
            label(t().common.preview, 11.0, style::SEMIBOLD, style::TEXT_3).width(100),
            space::horizontal().width(28),
        ]
        .spacing(8);
        let rows = self.labels.iter().enumerate().fold(
            Column::new().spacing(10),
            |rows, (index, draft)| {
                let chip: Element<'_, Message> = if let Some(rule) = &draft.parsed {
                    crate::panes::label_chip(&draft.name, 11.0, Some(rule))
                } else {
                    label(t().settings.label_invalid, 11.0, style::INTER, style::LIVE).into()
                };
                rows.push(
                    row![
                        self.label_input(
                            index,
                            LabelField::Name,
                            &draft.name,
                            draft.name.trim().is_empty()
                        )
                        .width(FillPortion(2)),
                        self.label_input(index, LabelField::Aliases, &draft.aliases, false)
                            .width(FillPortion(3)),
                        self.label_input(
                            index,
                            LabelField::Color,
                            &draft.color,
                            Rgb::from_hex(&draft.color).is_none()
                        )
                        .width(112),
                        self.label_input(
                            index,
                            LabelField::Shortcut,
                            &draft.shortcut,
                            !draft.shortcut.is_empty()
                                && (draft.shortcut.len() != 1
                                    || !draft.shortcut.as_bytes()[0].is_ascii_uppercase())
                        )
                        .width(70),
                        container(chip).width(100).clip(true),
                        ui::icon_button(
                            icon::TRASH_2,
                            style::TEXT_2,
                            28.0,
                            (!self.busy).then_some(Message::DeleteLabel(index))
                        ),
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
            },
        );
        container(
            column![
                ui::eyebrow(t().settings.labels),
                label(t().settings.labels_hint, 12.0, style::INTER, style::TEXT_2),
                header,
                ui::scroll(rows).height(Fill),
                action(
                    Tone::Ghost,
                    Some(icon::PLUS),
                    t().settings.add_label,
                    None,
                    (!self.busy && self.labels.len() < MAX_RULES).then_some(Message::AddLabel)
                ),
                label(t().settings.labels_limit, 11.0, style::INTER, style::TEXT_3),
            ]
            .spacing(12),
        )
        .padding([20, 24])
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn label_input<'a>(
        &self,
        index: usize,
        field: LabelField,
        value: &'a str,
        invalid: bool,
    ) -> iced::widget::TextInput<'a, Message> {
        text_input("", value)
            .on_input_maybe(
                (!self.busy).then_some(move |value| Message::LabelChanged(index, field, value)),
            )
            .on_submit(Message::Save)
            .padding([8, 8])
            .size(12)
            .style(if invalid {
                style::invalid_field
            } else {
                style::field
            })
    }
}

use super::style::{
    accent_icon_button_style, highlight_history_target, pressed_entry_button_style,
    transparent_entry_button_style, transparent_icon_button_style,
};
use super::summary::{summarize_one_line, summarize_one_line_with_limit};
use crate::app::model::{FocusPart, HistoryItem};
use crate::app::{AppModel, Message, icons};
use crate::fl;
use crate::services::clipboard;
use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub(super) struct RowRenderState {
    pub(super) idx: usize,
    pub(super) item: HistoryItem,
    pub(super) row_is_hovered: bool,
    pub(super) row_keyboard_focus: Option<FocusPart>,
    pub(super) hovered_focus: Option<FocusPart>,
    pub(super) thumbnail_handle: Option<ImageHandle>,
}

impl RowRenderState {
    pub(super) fn from_app(app: &AppModel, idx: usize, item: &HistoryItem) -> Self {
        let row_is_hovered = app.hovered_index == Some(idx);
        let row_keyboard_focus = app
            .keyboard_focus
            .and_then(|(focus_idx, part)| (focus_idx == idx).then_some(part));
        let hovered_focus = app
            .hovered_focus
            .and_then(|(focus_idx, part)| (focus_idx == idx).then_some(part));

        let thumbnail_handle = match &item.entry {
            clipboard::ClipboardEntry::Image { hash, bytes, .. } => {
                app.thumbnail_handles.get(&(*hash, bytes.len())).cloned()
            }
            clipboard::ClipboardEntry::Text(_) => None,
        };

        Self {
            idx,
            item: item.clone(),
            row_is_hovered,
            row_keyboard_focus,
            hovered_focus,
            thumbnail_handle,
        }
    }
}

impl Hash for RowRenderState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.item.pinned.hash(state);
        self.row_is_hovered.hash(state);
        self.row_keyboard_focus.hash(state);
        self.hovered_focus.hash(state);
        self.thumbnail_handle.is_some().hash(state);

        match &self.item.entry {
            clipboard::ClipboardEntry::Text(text) => {
                0u8.hash(state);
                text.hash(state);
            }
            clipboard::ClipboardEntry::Image {
                mime,
                bytes,
                hash,
                thumbnail_png,
            } => {
                1u8.hash(state);
                mime.hash(state);
                bytes.len().hash(state);
                hash.hash(state);
                thumbnail_png.as_ref().map(|p| p.len()).hash(state);
            }
        }
    }
}

pub(super) fn history_row(state: RowRenderState) -> Element<'static, Message> {
    const TEXT_EXPANDED_MAX_CHARS: usize = 300;
    const IMAGE_PREVIEW_COLLAPSED_HEIGHT: f32 = 160.0;
    const IMAGE_PREVIEW_EXPANDED_HEIGHT: f32 = 200.0;

    let row_expanded = state.row_is_hovered || state.row_keyboard_focus.is_some();
    let row_alignment = if matches!(&state.item.entry, clipboard::ClipboardEntry::Image { .. }) {
        Alignment::Start
    } else {
        Alignment::Center
    };

    let label: Element<'static, Message> = match &state.item.entry {
        clipboard::ClipboardEntry::Text(text) => {
            let summary = if row_expanded {
                summarize_one_line_with_limit(text, TEXT_EXPANDED_MAX_CHARS)
            } else {
                summarize_one_line(text)
            };
            widget::text::body(summary).into()
        }
        clipboard::ClipboardEntry::Image {
            mime,
            bytes,
            thumbnail_png,
            ..
        } => {
            let cached_handle = state.thumbnail_handle.clone();
            let thumb: Option<Element<'_, Message>> = thumbnail_png.as_ref().map(|png| {
                let preview_height = if row_expanded {
                    IMAGE_PREVIEW_EXPANDED_HEIGHT
                } else {
                    IMAGE_PREVIEW_COLLAPSED_HEIGHT
                };
                let handle = cached_handle
                    .clone()
                    .unwrap_or_else(|| ImageHandle::from_bytes(png.clone()));

                widget::container(
                    widget::image::<ImageHandle>(handle)
                        .width(Length::Fill)
                        .height(Length::Fixed(preview_height))
                        .content_fit(cosmic::iced::ContentFit::Contain)
                        .expand(false),
                )
                .width(Length::Fill)
                .height(Length::Fixed(preview_height))
                .max_height(preview_height)
                .clip(true)
                .into()
            });

            let mut col = widget::column::Column::new()
                .width(Length::Fill)
                .align_x(Alignment::Center);
            if let Some(thumb) = thumb {
                col = col.push(thumb);
            }
            let details = if row_expanded {
                format!(
                    "{} ({} KB)",
                    mime,
                    (bytes.len().saturating_add(1023)) / 1024
                )
            } else {
                "\u{00A0}".to_string()
            };

            col = col.push(widget::text::caption(details).width(Length::Fill));
            col.into()
        }
    };

    let copy_button = widget::button::custom(label)
        .class(cosmic::theme::Button::Custom {
            active: Box::new(|_, theme| transparent_entry_button_style(theme)),
            disabled: Box::new(transparent_entry_button_style),
            hovered: Box::new(|_, theme| transparent_entry_button_style(theme)),
            pressed: Box::new(|_, theme| pressed_entry_button_style(theme)),
        })
        .on_press(Message::CopyFromHistory(state.idx))
        .width(Length::Fill)
        .padding([8, 12]);

    let entry_active = state.row_keyboard_focus == Some(FocusPart::Entry)
        || state.hovered_focus == Some(FocusPart::Entry);

    let copy_button_elem = widget::mouse_area(highlight_history_target(
        widget::container(copy_button).width(Length::Fill).into(),
        entry_active,
    ));

    let pin_button_class = if state.item.pinned {
        cosmic::theme::Button::Custom {
            active: Box::new(|_, theme| accent_icon_button_style(theme)),
            disabled: Box::new(accent_icon_button_style),
            hovered: Box::new(|_, theme| accent_icon_button_style(theme)),
            pressed: Box::new(|_, theme| accent_icon_button_style(theme)),
        }
    } else {
        cosmic::theme::Button::Custom {
            active: Box::new(|_, theme| transparent_icon_button_style(theme)),
            disabled: Box::new(transparent_icon_button_style),
            hovered: Box::new(|_, theme| transparent_icon_button_style(theme)),
            pressed: Box::new(|_, theme| transparent_icon_button_style(theme)),
        }
    };

    let pin_button = widget::button::icon(if state.item.pinned {
        icons::pin_icon_pinned()
    } else {
        icons::pin_icon()
    })
    .class(pin_button_class)
    .tooltip(if state.item.pinned {
        fl!("unpin")
    } else {
        fl!("pin")
    })
    .on_press(Message::TogglePin(state.idx))
    .extra_small()
    .width(Length::Shrink);

    let remove_button = widget::button::icon(icons::remove_icon())
        .class(cosmic::theme::Button::Custom {
            active: Box::new(|_, theme| transparent_icon_button_style(theme)),
            disabled: Box::new(transparent_icon_button_style),
            hovered: Box::new(|_, theme| transparent_icon_button_style(theme)),
            pressed: Box::new(|_, theme| transparent_icon_button_style(theme)),
        })
        .tooltip(fl!("remove"))
        .on_press(Message::RemoveHistory(state.idx))
        .extra_small()
        .width(Length::Shrink);

    let pin_active = state.row_keyboard_focus == Some(FocusPart::Pin)
        || state.hovered_focus == Some(FocusPart::Pin);
    let remove_active = state.row_keyboard_focus == Some(FocusPart::Remove)
        || state.hovered_focus == Some(FocusPart::Remove);

    let pin_button_elem =
        widget::mouse_area(highlight_history_target(pin_button.into(), pin_active))
            .on_enter(Message::HoverEntry(Some((state.idx, FocusPart::Pin))))
            .on_exit(Message::HoverEntry(Some((state.idx, FocusPart::Entry))));

    let remove_button_elem = widget::mouse_area(highlight_history_target(
        remove_button.into(),
        remove_active,
    ))
    .on_enter(Message::HoverEntry(Some((state.idx, FocusPart::Remove))))
    .on_exit(Message::HoverEntry(Some((state.idx, FocusPart::Entry))));

    let actions = widget::column::Column::new()
        .spacing(2)
        .align_x(Alignment::Center)
        .push(pin_button_elem)
        .push(remove_button_elem);

    let entry = widget::row::Row::new()
        .push(copy_button_elem)
        .push(
            widget::container(actions)
                .width(Length::Fixed(40.0))
                .padding([0, 2]),
        )
        .align_y(row_alignment)
        .width(Length::Fill);

    let row_card = widget::container(entry)
        .class(cosmic::theme::Container::Card)
        .width(Length::Fill)
        .clip(true);

    widget::mouse_area(row_card)
        .on_enter(Message::HoverEntry(Some((state.idx, FocusPart::Entry))))
        .on_exit(Message::HoverEntry(None))
        .into()
}

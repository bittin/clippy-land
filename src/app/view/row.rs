use super::style::{
    accent_icon_button_style, highlight_history_target, pressed_entry_button_style,
    transparent_entry_button_style, transparent_icon_button_style,
};
use super::summary::{
    EXPANDED_MAX_CHARS, summarize_one_line, summarize_one_line_with_limit, text_overlay_available,
};
use crate::app::model::{FocusPart, HistoryItem};
use crate::app::{AppModel, Message, icons};
use crate::fl;
use crate::services::clipboard;
use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;
use std::hash::{Hash, Hasher};

const IMAGE_PREVIEW_HEIGHT: f32 = 200.0;

#[derive(Clone)]
pub(super) enum RowContent {
    Text {
        collapsed_summary: String,
        expanded_summary: String,
        overlay_available: bool,
    },
    Image {
        mime: String,
        bytes_len: usize,
        content_hash: u64,
        thumbnail_handle: Option<ImageHandle>,
    },
}

#[derive(Clone)]
pub(super) struct RowRenderState {
    pub(super) idx: usize,
    pub(super) pinned: bool,
    pub(super) text_overlay_open: bool,
    pub(super) row_is_hovered: bool,
    pub(super) row_keyboard_focus: Option<FocusPart>,
    pub(super) hovered_focus: Option<FocusPart>,
    pub(super) content: RowContent,
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

        let content = match &item.entry {
            clipboard::ClipboardEntry::Text(text) => RowContent::Text {
                collapsed_summary: summarize_one_line(text),
                expanded_summary: summarize_one_line_with_limit(text, EXPANDED_MAX_CHARS),
                overlay_available: text_overlay_available(text),
            },
            clipboard::ClipboardEntry::Image {
                mime,
                bytes,
                hash,
                thumbnail_png: _,
            } => RowContent::Image {
                mime: mime.clone(),
                bytes_len: bytes.len(),
                content_hash: *hash,
                thumbnail_handle: app.thumbnail_handles.get(&(*hash, bytes.len())).cloned(),
            },
        };

        Self {
            idx,
            pinned: item.pinned,
            text_overlay_open: app.text_overlay_index == Some(idx),
            row_is_hovered,
            row_keyboard_focus,
            hovered_focus,
            content,
        }
    }
}

impl Hash for RowRenderState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.pinned.hash(state);
        self.row_is_hovered.hash(state);
        self.row_keyboard_focus.hash(state);
        self.hovered_focus.hash(state);

        match &self.content {
            RowContent::Text {
                collapsed_summary,
                expanded_summary,
                overlay_available,
            } => {
                0u8.hash(state);
                collapsed_summary.hash(state);
                expanded_summary.hash(state);
                overlay_available.hash(state);
            }
            RowContent::Image {
                mime,
                bytes_len,
                content_hash,
                thumbnail_handle,
            } => {
                1u8.hash(state);
                mime.hash(state);
                bytes_len.hash(state);
                content_hash.hash(state);
                thumbnail_handle.is_some().hash(state);
            }
        }
    }
}

pub(super) fn history_row(state: RowRenderState) -> Element<'static, Message> {
    let row_expanded = state.row_is_hovered || state.row_keyboard_focus.is_some();
    let row_alignment = Alignment::Center;

    let label: Element<'static, Message> = match &state.content {
        RowContent::Text {
            collapsed_summary,
            expanded_summary: _,
            overlay_available: _,
        } => widget::text::body(collapsed_summary.clone()).into(),
        RowContent::Image {
            mime,
            bytes_len,
            thumbnail_handle,
            ..
        } => {
            let thumb: Option<Element<'_, Message>> = thumbnail_handle.clone().map(|handle| {
                widget::container(
                    widget::image::<ImageHandle>(handle)
                        .width(Length::Fill)
                        .height(Length::Fixed(IMAGE_PREVIEW_HEIGHT))
                        .content_fit(cosmic::iced::ContentFit::Contain)
                        .expand(false),
                )
                .width(Length::Fill)
                .height(Length::Fixed(IMAGE_PREVIEW_HEIGHT))
                .max_height(IMAGE_PREVIEW_HEIGHT)
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
                format!("{} ({} KB)", mime, (bytes_len.saturating_add(1023)) / 1024)
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

    let pin_button_class = if state.pinned {
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

    let pin_button = widget::button::icon(if state.pinned {
        icons::pin_icon_pinned()
    } else {
        icons::pin_icon()
    })
    .class(pin_button_class)
    .tooltip(if state.pinned {
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
    let preview_active = state.row_keyboard_focus == Some(FocusPart::Preview)
        || state.hovered_focus == Some(FocusPart::Preview)
        || state.text_overlay_open;
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

    let mut actions = widget::column::Column::new()
        .spacing(2)
        .align_x(Alignment::Center);

    if matches!(
        &state.content,
        RowContent::Text {
            overlay_available: true,
            ..
        }
    ) {
        let preview_button =
            widget::button::icon(widget::icon::from_name("system-search-symbolic"))
                .class(cosmic::theme::Button::Custom {
                    active: Box::new(|_, theme| transparent_icon_button_style(theme)),
                    disabled: Box::new(transparent_icon_button_style),
                    hovered: Box::new(|_, theme| transparent_icon_button_style(theme)),
                    pressed: Box::new(|_, theme| transparent_icon_button_style(theme)),
                })
                .tooltip("Preview full text")
                .on_press(Message::OpenTextOverlay(state.idx))
                .extra_small()
                .width(Length::Shrink);

        let preview_button_elem = widget::mouse_area(highlight_history_target(
            preview_button.into(),
            preview_active,
        ))
        .on_enter(Message::HoverEntry(Some((state.idx, FocusPart::Preview))))
        .on_exit(Message::HoverEntry(Some((state.idx, FocusPart::Entry))));

        actions = actions.push(preview_button_elem);
    }

    actions = actions.push(pin_button_elem).push(remove_button_elem);

    let entry = widget::row::Row::new()
        .push(copy_button_elem)
        .push(
            widget::container(actions)
                .width(Length::Fixed(44.0))
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

use super::super::super::style::{
    accent_icon_button_style, accent_svg_style, container_on_svg_style, highlight_history_target,
    transparent_icon_button_style,
};
use super::super::{RowContent, RowRenderState};
use crate::app::model::FocusPart;
use crate::app::{Message, icons};
use crate::fl;
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget;

pub(super) fn row_actions(state: &RowRenderState) -> Element<'static, Message> {
    let pin_active = state.row_keyboard_focus == Some(FocusPart::Pin)
        || state.hovered_focus == Some(FocusPart::Pin);
    let preview_active = state.row_keyboard_focus == Some(FocusPart::Preview)
        || state.hovered_focus == Some(FocusPart::Preview)
        || state.text_overlay_open;
    let remove_active = state.row_keyboard_focus == Some(FocusPart::Remove)
        || state.hovered_focus == Some(FocusPart::Remove);

    let mut actions = widget::column::Column::new()
        .spacing(2)
        .align_x(cosmic::iced::Alignment::Center);

    if matches!(
        &state.content,
        RowContent::Text {
            overlay_available: true,
            ..
        }
    ) {
        actions = actions.push(preview_button(state, preview_active));
    }

    actions = actions
        .push(pin_button(state, pin_active))
        .push(remove_button(state, remove_active));

    widget::container(actions)
        .width(Length::Fixed(44.0))
        .padding([0, 2])
        .into()
}

fn pin_button(state: &RowRenderState, active: bool) -> Element<'static, Message> {
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

    let pin_icon = widget::icon(if state.pinned {
        icons::pin_icon_pinned()
    } else {
        icons::pin_icon()
    })
    .class(if state.pinned {
        accent_svg_style()
    } else {
        container_on_svg_style()
    })
    .size(16);

    let pin_button = widget::button::custom(pin_icon)
        .class(pin_button_class)
        .on_press(Message::TogglePin(state.idx))
        .width(Length::Shrink);

    widget::mouse_area(highlight_history_target(
        widget::tooltip(
            pin_button,
            widget::text(if state.pinned {
                fl!("unpin")
            } else {
                fl!("pin")
            }),
            widget::tooltip::Position::Top,
        )
        .into(),
        active,
    ))
    .on_enter(Message::HoverEntry(Some((state.idx, FocusPart::Pin))))
    .on_exit(Message::HoverEntry(Some((state.idx, FocusPart::Entry))))
    .into()
}

fn remove_button(state: &RowRenderState, active: bool) -> Element<'static, Message> {
    let remove_button = widget::button::custom(
        widget::icon(icons::remove_icon())
            .class(container_on_svg_style())
            .size(16),
    )
    .class(cosmic::theme::Button::Custom {
        active: Box::new(|_, theme| transparent_icon_button_style(theme)),
        disabled: Box::new(transparent_icon_button_style),
        hovered: Box::new(|_, theme| transparent_icon_button_style(theme)),
        pressed: Box::new(|_, theme| transparent_icon_button_style(theme)),
    })
    .on_press(Message::RemoveHistory(state.idx))
    .width(Length::Shrink);

    widget::mouse_area(highlight_history_target(
        widget::tooltip(
            remove_button,
            widget::text(fl!("remove")),
            widget::tooltip::Position::Top,
        )
        .into(),
        active,
    ))
    .on_enter(Message::HoverEntry(Some((state.idx, FocusPart::Remove))))
    .on_exit(Message::HoverEntry(Some((state.idx, FocusPart::Entry))))
    .into()
}

fn preview_button(state: &RowRenderState, active: bool) -> Element<'static, Message> {
    let preview_button = widget::button::custom(
        widget::icon(icons::named_symbolic_icon("system-search-symbolic"))
            .class(container_on_svg_style())
            .size(16),
    )
    .class(cosmic::theme::Button::Custom {
        active: Box::new(|_, theme| transparent_icon_button_style(theme)),
        disabled: Box::new(transparent_icon_button_style),
        hovered: Box::new(|_, theme| transparent_icon_button_style(theme)),
        pressed: Box::new(|_, theme| transparent_icon_button_style(theme)),
    })
    .on_press(Message::OpenTextOverlay(state.idx))
    .width(Length::Shrink);

    widget::mouse_area(highlight_history_target(
        widget::tooltip(
            preview_button,
            widget::text("Preview full text"),
            widget::tooltip::Position::Top,
        )
        .into(),
        active,
    ))
    .on_enter(Message::HoverEntry(Some((state.idx, FocusPart::Preview))))
    .on_exit(Message::HoverEntry(Some((state.idx, FocusPart::Entry))))
    .into()
}

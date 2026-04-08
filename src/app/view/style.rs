use crate::app::Message;
use cosmic::iced::{Background, Border, Color};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::button::Style as ButtonStyle;
use cosmic::widget::container::Style as ContainerStyle;

pub(super) fn highlight_history_target<'a>(
    content: Element<'a, Message>,
    active: bool,
) -> Element<'a, Message> {
    if !active {
        return content;
    }

    widget::container(content)
        .class(cosmic::theme::Container::custom(|theme| {
            let c = theme.cosmic();
            // Use `on` (the contrasting color) at low alpha so the highlight is
            // visible in both light mode (dark-on-white) and dark mode (light-on-dark).
            // `component.hover` is pre-composited and nearly invisible in light mode.
            let on: Color = theme.current_container().component.on.into();
            ContainerStyle {
                background: Some(Background::Color(Color { a: 0.1, ..on })),
                border: Border {
                    radius: c.corner_radii.radius_s.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}

/// Transparent style for icon-only buttons. Suppresses the built-in hover
/// background so `highlight_history_target` is the sole visual indicator,
/// giving identical feedback for both mouse and keyboard navigation.
pub(super) fn transparent_icon_button_style(_theme: &cosmic::Theme) -> ButtonStyle {
    ButtonStyle::default()
}

pub(super) fn transparent_entry_button_style(theme: &cosmic::Theme) -> ButtonStyle {
    let c = theme.cosmic();
    let on_color: Color = theme.current_container().component.on.into();

    ButtonStyle {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border_radius: c.corner_radii.radius_s.into(),
        icon_color: Some(on_color),
        text_color: Some(on_color),
        ..Default::default()
    }
}

/// Pressed variant: stronger highlight using the `on` color at higher alpha.
/// Iced applies this automatically while the button is held down.
pub(super) fn pressed_entry_button_style(theme: &cosmic::Theme) -> ButtonStyle {
    let c = theme.cosmic();
    let on_color: Color = theme.current_container().component.on.into();

    ButtonStyle {
        background: Some(Background::Color(Color { a: 0.2, ..on_color })),
        border_radius: c.corner_radii.radius_s.into(),
        icon_color: Some(on_color),
        text_color: Some(on_color),
        ..Default::default()
    }
}

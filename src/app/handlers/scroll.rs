use crate::app::{AppModel, Message};
use cosmic::iced::widget::scrollable::{self, AbsoluteOffset, RelativeOffset, Viewport};
use cosmic::prelude::*;

const TEXT_OVERLAY_KEYBOARD_SCROLL_DELTA: f32 = 48.0;

pub(super) fn scroll_text_overlay_up() -> Task<cosmic::Action<Message>> {
    scroll_text_overlay_by(-TEXT_OVERLAY_KEYBOARD_SCROLL_DELTA)
}

pub(super) fn scroll_text_overlay_down() -> Task<cosmic::Action<Message>> {
    scroll_text_overlay_by(TEXT_OVERLAY_KEYBOARD_SCROLL_DELTA)
}

fn scroll_text_overlay_by(delta_y: f32) -> Task<cosmic::Action<Message>> {
    scrollable::scroll_by(
        crate::app::text_overlay_scroll_id(),
        AbsoluteOffset { x: 0.0, y: delta_y },
    )
}

pub(super) fn scroll_selection_into_view(
    app: &AppModel,
    selected_index: usize,
) -> Task<cosmic::Action<Message>> {
    let current_top = app
        .history_viewport
        .map(|viewport| viewport.relative_offset().y);
    let visible_fraction = app.history_viewport.map(visible_vertical_fraction);

    desired_scroll_y(
        current_top,
        visible_fraction,
        selected_index,
        app.history.len(),
    )
    .map(|y| {
        scrollable::snap_to(
            crate::app::history_scroll_id(),
            RelativeOffset { x: 0.0, y }.into(),
        )
    })
    .unwrap_or_else(Task::none)
}

fn visible_vertical_fraction(viewport: Viewport) -> f32 {
    let bounds = viewport.bounds();
    let content_bounds = viewport.content_bounds();

    if content_bounds.height <= 0.0 {
        1.0
    } else {
        (bounds.height / content_bounds.height).clamp(0.0, 1.0)
    }
}

pub(super) fn desired_scroll_y(
    current_top: Option<f32>,
    visible_fraction: Option<f32>,
    selected_index: usize,
    item_count: usize,
) -> Option<f32> {
    if item_count <= 1 {
        return None;
    }

    let row_fraction = 1.0 / item_count as f32;
    let target_center = (selected_index.min(item_count - 1) as f32 + 0.5) * row_fraction;

    let (Some(current_top), Some(visible_fraction)) = (current_top, visible_fraction) else {
        return Some(target_center.clamp(0.0, 1.0));
    };

    if !current_top.is_finite() || !visible_fraction.is_finite() {
        return Some(target_center.clamp(0.0, 1.0));
    }

    let visible_fraction = visible_fraction.clamp(0.0, 1.0);
    if visible_fraction >= 0.999 {
        return None;
    }

    let scrollable_fraction = (1.0 - visible_fraction).max(f32::EPSILON);
    let desired_top =
        ((target_center - (visible_fraction / 2.0)) / scrollable_fraction).clamp(0.0, 1.0);

    if (current_top - desired_top).abs() <= (row_fraction / scrollable_fraction) / 3.0 {
        None
    } else {
        Some(desired_top)
    }
}

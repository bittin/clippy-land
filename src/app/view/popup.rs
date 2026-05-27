use super::row::{RowRenderState, history_row};
use crate::app::{AppModel, Message, icons};
use crate::fl;
use cosmic::iced::{Alignment, Length, window::Id};
use cosmic::prelude::*;
use cosmic::widget;

pub(super) fn view(app: &AppModel) -> Element<'_, Message> {
    app.core
        .applet
        .icon_button("edit-copy-symbolic")
        .on_press(Message::TogglePopup)
        .into()
}

/// Returns the indices into `app.history` that match the current search query.
pub(crate) fn filtered_indices(app: &AppModel) -> Vec<usize> {
    let cache_looks_valid = app.filtered_query_cache == app.search_query
        && app.filtered_history_len_cache == app.history.len()
        && app
            .filtered_indices
            .iter()
            .all(|&idx| idx < app.history.len());

    if cache_looks_valid {
        app.filtered_indices.clone()
    } else {
        AppModel::compute_filtered_indices_for(&app.history, &app.search_query)
    }
}

pub(super) fn view_window(app: &AppModel, _id: Id) -> Element<'_, Message> {
    let visible = filtered_indices(app);
    let mut history_column = widget::column::Column::new().spacing(4);

    if app.history.is_empty() {
        history_column = history_column.push(
            widget::container(widget::text::body(fl!("empty")))
                .width(Length::Fill)
                .center_x(Length::Fill),
        );
    } else if visible.is_empty() {
        history_column = history_column.push(
            widget::container(widget::text::body(fl!("no-results")))
                .width(Length::Fill)
                .center_x(Length::Fill),
        );
    } else {
        let pinned_count = app.history.iter().filter(|it| it.pinned).count();

        for &idx in &visible {
            // Show divider between pinned and unpinned sections when not filtering
            if app.search_query.is_empty()
                && idx == pinned_count
                && pinned_count > 0
                && pinned_count < app.history.len()
            {
                history_column = history_column.push(widget::divider::horizontal::default());
            }

            let row_state = RowRenderState::from_app(app, idx, &app.history[idx]);
            history_column = history_column.push(history_row(row_state));
        }
    }

    let history_scrollable = widget::container(
        widget::scrollable(
            widget::container(history_column)
                .padding([0, 12, 0, 12])
                .width(Length::Fill),
        )
        .id(crate::app::history_scroll_id())
        .on_scroll(Message::HistoryScrolled)
        .width(Length::Fill),
    )
    .max_height(400.0)
    .clip(true)
    .width(Length::Fill);

    let search_bar = widget::container(
        widget::search_input(fl!("search-placeholder"), &app.search_query)
            .on_input(Message::SearchChanged)
            .on_clear(Message::SearchChanged(String::new()))
            .width(Length::Fill),
    )
    .padding([0, 12]);

    let mut content = widget::column::Column::new().spacing(8).padding([8, 8]);

    if app.settings_open {
        let settings_form = settings_panel(app);
        content = content.push(widget::container(settings_form).padding([0, 12]));
    } else {
        if !app.history.is_empty() {
            content = content.push(search_bar);
        }

        content = content.push(history_scrollable);
    }

    let mut controls = widget::row::Row::new()
        .spacing(8)
        .align_y(Alignment::Center);

    let settings_button =
        widget::button::icon(widget::icon::from_name("preferences-system-symbolic"))
            .tooltip(if app.settings_open {
                "Close settings"
            } else {
                "Settings"
            })
            .on_press(Message::ToggleSettingsPanel)
            .extra_small()
            .width(Length::Shrink);
    controls = controls.push(settings_button);

    controls = controls.push(widget::space().width(Length::Fill));

    if !app.history.is_empty() && app.search_query.is_empty() {
        let delete_all_button = widget::button::destructive(fl!("delete-all"))
            .leading_icon(icons::remove_icon())
            .on_press(Message::ClearHistory);
        controls = controls.push(delete_all_button);
    }

    let controls_sheet = widget::container(controls)
        .padding([8, 8])
        .width(Length::Fill);
    content = content.push(controls_sheet);

    app.core.applet.popup_container(content).into()
}

fn settings_panel(app: &AppModel) -> Element<'_, Message> {
    let mut col = widget::column::Column::new().spacing(8).width(Length::Fill);

    col = col
        .push(widget::text::heading("Settings"))
        .push(widget::text::caption("History limits"));

    let history_max = widget::column::Column::new()
        .spacing(4)
        .push(widget::text::body("Max history entries"))
        .push(
            widget::text_input("e.g. 200", &app.settings_draft.max_history)
                .on_input(Message::SettingsMaxHistoryChanged)
                .width(Length::Fill),
        )
        .push(widget::text::caption("Allowed range: 30–5000"));

    let pinned_max = widget::column::Column::new()
        .spacing(4)
        .push(widget::text::body("Max pinned entries"))
        .push(
            widget::text_input("e.g. 20", &app.settings_draft.max_pinned)
                .on_input(Message::SettingsMaxPinnedChanged)
                .width(Length::Fill),
        )
        .push(widget::text::caption(
            "Allowed range: 0–500 (and ≤ max history)",
        ));

    col = col.push(
        widget::row::Row::new()
            .spacing(8)
            .push(history_max)
            .push(pinned_max),
    );

    col = col.push(widget::divider::horizontal::light());
    col = col.push(widget::text::caption("Image limits"));

    let image_bytes = widget::column::Column::new()
        .spacing(4)
        .push(widget::text::body("Max image size (bytes)"))
        .push(
            widget::text_input("e.g. 8388608", &app.settings_draft.max_image_bytes)
                .on_input(Message::SettingsMaxImageBytesChanged)
                .width(Length::Fill),
        )
        .push(widget::text::caption("Allowed range: 262144–67108864"));

    let image_dimension = widget::column::Column::new()
        .spacing(4)
        .push(widget::text::body("Max image dimension (px)"))
        .push(
            widget::text_input("e.g. 8192", &app.settings_draft.max_image_dimension_px)
                .on_input(Message::SettingsMaxImageDimensionChanged)
                .width(Length::Fill),
        )
        .push(widget::text::caption("Allowed range: 512–16384"));

    col = col.push(
        widget::row::Row::new()
            .spacing(8)
            .push(image_bytes)
            .push(image_dimension),
    );

    if let Some(err) = &app.settings_error {
        col = col.push(widget::text::body(err));
    }

    let apply_row = widget::row::Row::new()
        .width(Length::Fill)
        .push(widget::button::suggested("Apply").on_press(Message::ApplySettings));

    col = col.push(apply_row);

    widget::container(col)
        .class(cosmic::theme::Container::Card)
        .padding([8, 12])
        .width(Length::Fill)
        .into()
}

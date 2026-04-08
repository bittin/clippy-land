use super::row::history_row;
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

pub(super) fn view_window(app: &AppModel, _id: Id) -> Element<'_, Message> {
    let mut history_column = widget::column::Column::new().spacing(4);

    if app.history.is_empty() {
        history_column = history_column.push(
            widget::container(widget::text::body(fl!("empty")))
                .width(Length::Fill)
                .center_x(Length::Fill),
        );
    } else {
        let pinned_count = app.history.iter().filter(|it| it.pinned).count();

        for (idx, item) in app.history.iter().enumerate() {
            if idx == pinned_count && pinned_count > 0 && pinned_count < app.history.len() {
                history_column = history_column.push(widget::divider::horizontal::default());
            }

            history_column = history_column.push(history_row(app, idx, item));
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
    .width(Length::Fill);

    let mut content = widget::column::Column::new()
        .spacing(8)
        .padding([8, 8])
        .push(history_scrollable);

    if !app.history.is_empty() {
        let delete_all_button = widget::button::destructive(fl!("delete-all"))
            .leading_icon(icons::remove_icon())
            .on_press(Message::ClearHistory);

        let controls_sheet = widget::container(delete_all_button)
            .padding([8, 8])
            .align_x(Alignment::End)
            .width(Length::Fill);
        content = content.push(controls_sheet);
    }

    app.core.applet.popup_container(content).into()
}

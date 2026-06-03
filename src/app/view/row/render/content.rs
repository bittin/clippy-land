use super::super::RowContent;
use super::super::state::RowRenderState;
use crate::app::Message;
use cosmic::iced::Length;
use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::prelude::*;
use cosmic::widget;

const IMAGE_PREVIEW_HEIGHT: f32 = 200.0;

pub(super) fn row_label(state: &RowRenderState, row_expanded: bool) -> Element<'static, Message> {
    match &state.content {
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
                .align_x(cosmic::iced::Alignment::Center);
            if let Some(thumb) = thumb {
                col = col.push(thumb);
            }

            let details = if row_expanded {
                format!("{} ({} KB)", mime, (bytes_len.saturating_add(1023)) / 1024)
            } else {
                "\u{00A0}".to_string()
            };

            col.push(widget::text::caption(details).width(Length::Fill))
                .into()
        }
    }
}

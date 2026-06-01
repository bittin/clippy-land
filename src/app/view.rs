mod popup;
mod row;
mod style;
pub(crate) mod summary;

#[cfg(test)]
mod tests;

pub(super) use popup::filtered_indices;

use super::{AppModel, Message};
use cosmic::iced::window::Id;
use cosmic::prelude::*;

pub(super) fn view(app: &AppModel) -> Element<'_, Message> {
    popup::view(app)
}

pub(super) fn view_window(app: &AppModel, id: Id) -> Element<'_, Message> {
    popup::view_window(app, id)
}

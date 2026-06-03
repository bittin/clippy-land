mod history;
mod scroll;
mod subscription;
mod update;

#[cfg(test)]
mod tests;

use super::{AppModel, Message};
use cosmic::iced::Subscription;
use cosmic::prelude::*;

pub(super) fn subscription(app: &AppModel) -> Subscription<Message> {
    subscription::subscription(app)
}

pub(super) fn update(app: &mut AppModel, message: Message) -> Task<cosmic::Action<Message>> {
    update::update(app, message)
}

pub(super) fn prewarm_for_first_popup(app: &mut AppModel) {
    update::warm_thumbnail_handles(app);
}

mod handlers;
mod icons;
mod messages;
mod model;
mod view;

pub use messages::Message;
pub use model::AppModel;
#[derive(Debug, Clone, Copy, Default)]
pub struct AppFlags {
    pub open_popup_on_start: bool,
}

use crate::app::model::SettingsDraft;
use crate::services::clipboard;
use crate::settings::AppSettings;
use cosmic::iced::platform_specific::shell::wayland::commands::layer_surface::{
    self, KeyboardInteractivity, get_layer_surface,
};
use cosmic::iced::runtime::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings;
use cosmic::iced::{Limits, Subscription, window::Id};
use cosmic::prelude::*;

pub(super) fn history_scroll_id() -> cosmic::iced::core::widget::Id {
    cosmic::iced::core::widget::Id::new("history-scroll")
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;

    type Flags = AppFlags;

    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format
    const APP_ID: &'static str = "io.github.k33wee.clippy-land";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(core: cosmic::Core, flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let settings = AppSettings::load().normalized();
        clipboard::configure_limits(settings.max_image_bytes, settings.max_image_dimension_px);

        let mut app = AppModel {
            core,
            settings_draft: SettingsDraft::from_settings(&settings),
            settings,
            ..Default::default()
        };
        app.recompute_filtered_indices();

        if flags.open_popup_on_start {
            app.begin_popup_open_trace("startup");
            let new_id = cosmic::iced::window::Id::unique();
            app.popup = Some(new_id);
            app.popup_is_layer_surface = true;

            (
                app,
                get_layer_surface(SctkLayerSurfaceSettings {
                    id: new_id,
                    keyboard_interactivity: KeyboardInteractivity::OnDemand,
                    anchor: layer_surface::Anchor::TOP
                        | layer_surface::Anchor::LEFT
                        | layer_surface::Anchor::RIGHT,
                    namespace: "clippy-land".into(),
                    size: Some((None, Some(400))),
                    size_limits: Limits::NONE.min_width(1.0).min_height(1.0),
                    ..Default::default()
                }),
            )
        } else {
            (app, Task::none())
        }
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Describes the interface based on the current state of the application model
    fn view(&self) -> Element<'_, Self::Message> {
        view::view(self)
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        view::view_window(self, _id)
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        handlers::subscription(self)
    }

    /// Handles messages emitted by the application and its widgets
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        handlers::update(self, message)
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

use super::shared::warm_thumbnail_handles;
use crate::app::model::PopupSurface;
use crate::app::{AppModel, Message};
use cosmic::iced::platform_specific::shell::wayland::commands::layer_surface::{
    destroy_layer_surface, get_layer_surface,
};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use std::time::Instant;

pub(super) fn handle(
    app: &mut AppModel,
    message: Message,
) -> Option<Task<cosmic::Action<Message>>> {
    match message {
        Message::TogglePopup => Some(toggle_popup(app)),
        Message::ToggleViaIpc => Some(toggle_via_ipc(app)),
        Message::PopupOpened(id) => {
            if app.popup.as_ref() == Some(&id) {
                app.popup_controls_ready = true;
                app.note_popup_stage_marker("popup controls ready after popup open");
                app.note_popup_opened();
            }
            Some(Task::none())
        }
        Message::PopupRedraw(id) => {
            if app.popup.as_ref() == Some(&id) {
                app.note_popup_stage_marker("first popup redraw observed");
                app.finish_popup_open_trace_on_redraw();
            }
            Some(Task::none())
        }
        Message::WindowUnfocused(id) => Some(window_unfocused(app, id)),
        Message::PopupClosed(id) => {
            if app.popup.as_ref() == Some(&id) {
                clear_popup_state(app, "popup closed before first redraw");
            }
            Some(Task::none())
        }
        _ => None,
    }
}

pub(super) fn warm_for_first_popup(app: &mut AppModel) {
    warm_thumbnail_handles(app);
}

fn toggle_popup(app: &mut AppModel) -> Task<cosmic::Action<Message>> {
    if app.popup.is_some() {
        close_popup(app, "popup toggled closed before first view")
    } else {
        app.begin_popup_open_trace("icon-click");
        open_anchored_popup(app)
    }
}

fn toggle_via_ipc(app: &mut AppModel) -> Task<cosmic::Action<Message>> {
    if app.popup.is_some() {
        close_popup(app, "ipc toggle closed popup before first view")
    } else {
        app.begin_popup_open_trace("ipc-toggle");
        let warm_started = Instant::now();
        warm_thumbnail_handles(app);
        app.note_popup_stage_duration("warm_thumbnail_handles complete", warm_started.elapsed());
        open_layer_surface_popup(app)
    }
}

fn open_layer_surface_popup(app: &mut AppModel) -> Task<cosmic::Action<Message>> {
    let new_id = cosmic::iced::window::Id::unique();
    app.popup.replace(new_id);
    app.popup_surface = Some(PopupSurface::LayerSurface);
    app.popup_controls_ready = false;
    app.note_popup_stage_marker("issuing get_layer_surface request");
    get_layer_surface(crate::app::history_layer_surface_settings(new_id))
}

fn open_anchored_popup(app: &mut AppModel) -> Task<cosmic::Action<Message>> {
    let new_id = cosmic::iced::window::Id::unique();
    app.popup.replace(new_id);
    app.popup_surface = Some(PopupSurface::AnchoredPopup);
    app.popup_controls_ready = false;

    let parent = app
        .core
        .main_window_id()
        .unwrap_or(cosmic::iced::window::Id::RESERVED);
    let popup_settings = app
        .core
        .applet
        .get_popup_settings(parent, new_id, None, None, None);

    app.note_popup_stage_marker("issuing get_popup request");
    get_popup(popup_settings)
}

fn close_popup(app: &mut AppModel, reason: &'static str) -> Task<cosmic::Action<Message>> {
    let Some(id) = app.popup.take() else {
        return Task::none();
    };

    let surface = app.popup_surface.take();

    clear_popup_state(app, reason);

    match surface {
        Some(PopupSurface::AnchoredPopup) => destroy_popup(id),
        Some(PopupSurface::LayerSurface) | None => destroy_layer_surface(id),
    }
}

fn clear_popup_state(app: &mut AppModel, reason: &'static str) {
    app.popup = None;
    app.popup_surface = None;
    app.popup_controls_ready = false;
    app.search_query.clear();
    app.settings_open = false;
    app.settings_error = None;
    app.hovered_index = None;
    app.at_scroll_bottom = false;
    app.history_viewport = None;
    app.text_overlay_index = None;
    app.cancel_popup_open_trace(reason);
}

fn window_unfocused(
    app: &mut AppModel,
    id: cosmic::iced::window::Id,
) -> Task<cosmic::Action<Message>> {
    if app.popup.as_ref() == Some(&id) && app.popup_surface == Some(PopupSurface::LayerSurface) {
        close_popup(app, "window lost focus before first redraw")
    } else {
        Task::none()
    }
}

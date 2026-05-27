use super::{history, scroll};
use crate::app::model::{FocusPart, HistoryItem, SettingsDraft};
use crate::app::view::filtered_indices;
use crate::app::{AppModel, Message};
use crate::services::clipboard::{self, ClipboardEntry};
use crate::settings::AppSettings;
use cosmic::iced::Limits;
use cosmic::iced::platform_specific::shell::wayland::commands::layer_surface::{
    self, KeyboardInteractivity, destroy_layer_surface, get_layer_surface,
};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::runtime::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings;
use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::prelude::*;

pub(super) fn update(app: &mut AppModel, message: Message) -> Task<cosmic::Action<Message>> {
    match message {
        Message::ClipboardChanged(entry) => {
            if app
                .history
                .front()
                .is_some_and(|it: &HistoryItem| &it.entry == &entry)
            {
                return Task::none();
            }

            if let ClipboardEntry::Text(text) = &entry {
                if history::should_ignore_clipboard_entry(text) {
                    return Task::none();
                }
            }

            cache_thumbnail_handle(app, &entry);

            let pinned = app
                .history
                .iter()
                .position(|it| &it.entry == &entry)
                .and_then(|idx| app.history.remove(idx))
                .is_some_and(|it| it.pinned);

            history::insert_after_pins(&mut app.history, HistoryItem { entry, pinned });
            history::trim_history(&mut app.history, &app.settings);
            prune_thumbnail_handles(app);
            app.recompute_filtered_indices();
        }
        Message::TogglePin(index) => {
            history::toggle_pin(&mut app.history, index, &app.settings);
            app.recompute_filtered_indices();
        }
        Message::CopyFromHistory(index) => {
            if let Some(item) = app.history.get(index) {
                history::copy_history_item(item);
            }
        }
        Message::RemoveHistory(index) => {
            let _ = app.history.remove(index);
            prune_thumbnail_handles(app);
            app.recompute_filtered_indices();
        }
        Message::ClearHistory => {
            app.history.clear();
            app.thumbnail_handles.clear();
            app.recompute_filtered_indices();
        }
        Message::HoverEntry(opt) => {
            let next_index = opt.map(|(idx, _)| idx);
            if app.hovered_index == next_index && app.hovered_focus == opt {
                return Task::none();
            }

            if let Some((idx, part)) = opt {
                app.hovered_index = Some(idx);
                app.hovered_focus = Some((idx, part));
                app.keyboard_focus = None;
            } else {
                app.hovered_index = None;
                app.hovered_focus = None;
            }
        }
        Message::HistoryScrolled(viewport) => {
            app.at_scroll_bottom = viewport.relative_offset().y >= 0.999;
            app.history_viewport = Some(viewport);
        }
        Message::MoveSelectionUp => {
            let visible = filtered_indices(app);
            if visible.is_empty() {
                return Task::none();
            }
            let new_idx = match app
                .hovered_index
                .and_then(|h| visible.iter().position(|&i| i == h))
            {
                Some(pos) => visible[if pos == 0 { visible.len() - 1 } else { pos - 1 }],
                None => *visible.last().unwrap(),
            };
            app.hovered_index = Some(new_idx);
            app.hovered_focus = None;
            app.keyboard_focus = Some((new_idx, FocusPart::Entry));
            app.at_scroll_bottom = false;
            return scroll::scroll_selection_into_view(app, new_idx);
        }
        Message::MoveSelectionDown => {
            let visible = filtered_indices(app);
            if visible.is_empty() {
                return Task::none();
            }
            let new_idx = match app
                .hovered_index
                .and_then(|h| visible.iter().position(|&i| i == h))
            {
                Some(pos) => visible[(pos + 1) % visible.len()],
                None => visible[0],
            };
            app.hovered_index = Some(new_idx);
            app.hovered_focus = None;
            app.keyboard_focus = Some((new_idx, FocusPart::Entry));
            app.at_scroll_bottom = false;
            return scroll::scroll_selection_into_view(app, new_idx);
        }
        Message::MoveFocusLeft => {
            if let Some((idx, part)) = app.keyboard_focus {
                if Some(idx) != app.hovered_index {
                    if let Some(h) = app.hovered_index {
                        app.keyboard_focus = Some((h, FocusPart::Entry));
                    }
                } else {
                    let new_part = match part {
                        FocusPart::Entry => FocusPart::Remove,
                        FocusPart::Pin => FocusPart::Entry,
                        FocusPart::Remove => FocusPart::Pin,
                    };
                    app.keyboard_focus = Some((idx, new_part));
                }
            } else if let Some(h) = app.hovered_index {
                app.keyboard_focus = Some((h, FocusPart::Entry));
            }
        }
        Message::MoveFocusRight => {
            if let Some((idx, part)) = app.keyboard_focus {
                if Some(idx) != app.hovered_index {
                    if let Some(h) = app.hovered_index {
                        app.keyboard_focus = Some((h, FocusPart::Entry));
                    }
                } else {
                    let new_part = match part {
                        FocusPart::Entry => FocusPart::Pin,
                        FocusPart::Pin => FocusPart::Remove,
                        FocusPart::Remove => FocusPart::Entry,
                    };
                    app.keyboard_focus = Some((idx, new_part));
                }
            } else if let Some(h) = app.hovered_index {
                app.keyboard_focus = Some((h, FocusPart::Entry));
            }
        }
        Message::ActivateSelection => {
            if let Some((idx, part)) = app.keyboard_focus {
                match part {
                    FocusPart::Entry => {
                        if let Some(item) = app.history.get(idx) {
                            history::copy_history_item(item);
                        }
                    }
                    FocusPart::Pin => {
                        history::toggle_pin(&mut app.history, idx, &app.settings);
                        app.recompute_filtered_indices();
                    }
                    FocusPart::Remove => {
                        let _ = app.history.remove(idx);
                        prune_thumbnail_handles(app);
                        app.recompute_filtered_indices();
                    }
                }
            } else if let Some(idx) = app.hovered_index {
                if let Some(item) = app.history.get(idx) {
                    history::copy_history_item(item);
                }
            }
        }
        Message::SearchChanged(query) => {
            app.search_query = query;
            app.recompute_filtered_indices();
            app.hovered_index = None;
            app.hovered_focus = None;
            app.keyboard_focus = None;
        }
        Message::ToggleSettingsPanel => {
            app.settings_open = !app.settings_open;
            app.settings_error = None;
            if app.settings_open {
                app.settings_draft = SettingsDraft::from_settings(&app.settings);
            }
        }
        Message::SettingsMaxHistoryChanged(value) => {
            app.settings_draft.max_history = value;
        }
        Message::SettingsMaxPinnedChanged(value) => {
            app.settings_draft.max_pinned = value;
        }
        Message::SettingsMaxImageBytesChanged(value) => {
            app.settings_draft.max_image_bytes = value;
        }
        Message::SettingsMaxImageDimensionChanged(value) => {
            app.settings_draft.max_image_dimension_px = value;
        }
        Message::ApplySettings => {
            let max_history = match parse_usize_field(&app.settings_draft.max_history) {
                Ok(v) => v,
                Err(err) => {
                    app.settings_error = Some(format!("Max history: {err}"));
                    return Task::none();
                }
            };
            let max_pinned = match parse_usize_field(&app.settings_draft.max_pinned) {
                Ok(v) => v,
                Err(err) => {
                    app.settings_error = Some(format!("Max pinned: {err}"));
                    return Task::none();
                }
            };
            let max_image_bytes = match parse_usize_field(&app.settings_draft.max_image_bytes) {
                Ok(v) => v,
                Err(err) => {
                    app.settings_error = Some(format!("Max image bytes: {err}"));
                    return Task::none();
                }
            };
            let max_image_dimension_px =
                match parse_u32_field(&app.settings_draft.max_image_dimension_px) {
                    Ok(v) => v,
                    Err(err) => {
                        app.settings_error = Some(format!("Max image dimension: {err}"));
                        return Task::none();
                    }
                };

            if !(AppSettings::MIN_HISTORY..=AppSettings::MAX_HISTORY).contains(&max_history) {
                app.settings_error = Some(format!(
                    "Max history must be between {} and {}",
                    AppSettings::MIN_HISTORY,
                    AppSettings::MAX_HISTORY
                ));
                return Task::none();
            }

            if !(AppSettings::MIN_PINNED..=AppSettings::MAX_PINNED).contains(&max_pinned) {
                app.settings_error = Some(format!(
                    "Max pinned must be between {} and {}",
                    AppSettings::MIN_PINNED,
                    AppSettings::MAX_PINNED
                ));
                return Task::none();
            }

            if max_pinned > max_history {
                app.settings_error = Some("Max pinned cannot be greater than max history".into());
                return Task::none();
            }

            if !(AppSettings::MIN_IMAGE_BYTES..=AppSettings::MAX_IMAGE_BYTES)
                .contains(&max_image_bytes)
            {
                app.settings_error = Some(format!(
                    "Max image bytes must be between {} and {}",
                    AppSettings::MIN_IMAGE_BYTES,
                    AppSettings::MAX_IMAGE_BYTES
                ));
                return Task::none();
            }

            if !(AppSettings::MIN_IMAGE_DIMENSION_PX..=AppSettings::MAX_IMAGE_DIMENSION_PX)
                .contains(&max_image_dimension_px)
            {
                app.settings_error = Some(format!(
                    "Max image dimension must be between {} and {}",
                    AppSettings::MIN_IMAGE_DIMENSION_PX,
                    AppSettings::MAX_IMAGE_DIMENSION_PX
                ));
                return Task::none();
            }

            let updated = AppSettings {
                schema_version: 1,
                max_history,
                max_pinned,
                max_image_bytes,
                max_image_dimension_px,
            }
            .normalized();

            if let Err(err) = updated.save() {
                app.settings_error = Some(format!("Failed to save settings: {err}"));
                return Task::none();
            }

            app.settings = updated;
            app.settings_draft = SettingsDraft::from_settings(&app.settings);
            app.settings_error = None;
            app.settings_open = false;

            clipboard::configure_limits(
                app.settings.max_image_bytes,
                app.settings.max_image_dimension_px,
            );
            history::reconcile_limits(&mut app.history, &app.settings);
            prune_thumbnail_handles(app);
            app.recompute_filtered_indices();
        }
        Message::TogglePopup => {
            return if let Some(p) = app.popup.take() {
                let is_layer = app.popup_is_layer_surface;
                app.popup_is_layer_surface = false;
                app.search_query.clear();
                app.settings_open = false;
                app.settings_error = None;
                if is_layer {
                    destroy_layer_surface(p)
                } else {
                    destroy_popup(p)
                }
            } else {
                let new_id = cosmic::iced::window::Id::unique();
                app.popup.replace(new_id);
                app.popup_is_layer_surface = false;
                let popup_settings = app.core.applet.get_popup_settings(
                    app.core.main_window_id().unwrap(),
                    new_id,
                    None,
                    None,
                    None,
                );
                get_popup(popup_settings)
            };
        }
        Message::ToggleViaIpc => {
            return if let Some(p) = app.popup.take() {
                let is_layer = app.popup_is_layer_surface;
                app.popup_is_layer_surface = false;
                app.search_query.clear();
                app.settings_open = false;
                app.settings_error = None;
                if is_layer {
                    destroy_layer_surface(p)
                } else {
                    destroy_popup(p)
                }
            } else {
                let new_id = cosmic::iced::window::Id::unique();
                app.popup.replace(new_id);
                app.popup_is_layer_surface = true;
                get_layer_surface(SctkLayerSurfaceSettings {
                    id: new_id,
                    keyboard_interactivity: KeyboardInteractivity::OnDemand,
                    // The anchor is set to TOP | LEFT | RIGHT to make the layer surface span the entire width of the screen and be positioned at the top, similar to a notification or a panel.
                    // Currently there is no way to follow the icon position in cosmic panel
                    anchor: layer_surface::Anchor::TOP
                        | layer_surface::Anchor::LEFT
                        | layer_surface::Anchor::RIGHT,
                    namespace: "clippy-land".into(),
                    size: Some((None, Some(400))),
                    size_limits: Limits::NONE.min_width(1.0).min_height(1.0),
                    ..Default::default()
                })
            };
        }
        Message::WindowUnfocused(id) => {
            if app.popup.as_ref() == Some(&id) && app.popup_is_layer_surface {
                return if let Some(p) = app.popup.take() {
                    app.popup_is_layer_surface = false;
                    app.search_query.clear();
                    app.settings_open = false;
                    app.settings_error = None;
                    app.hovered_index = None;
                    app.at_scroll_bottom = false;
                    app.history_viewport = None;
                    destroy_layer_surface(p)
                } else {
                    Task::none()
                };
            }
        }
        Message::PopupClosed(id) => {
            if app.popup.as_ref() == Some(&id) {
                app.popup = None;
                app.popup_is_layer_surface = false;
                app.search_query.clear();
                app.settings_open = false;
                app.settings_error = None;
                app.hovered_index = None;
                app.at_scroll_bottom = false;
                app.history_viewport = None;
            }
        }
    }
    Task::none()
}

fn cache_thumbnail_handle(app: &mut AppModel, entry: &ClipboardEntry) {
    let ClipboardEntry::Image {
        bytes,
        hash,
        thumbnail_png: Some(thumbnail_png),
        ..
    } = entry
    else {
        return;
    };

    app.thumbnail_handles
        .entry((*hash, bytes.len()))
        .or_insert_with(|| ImageHandle::from_bytes(thumbnail_png.clone()));
}

fn prune_thumbnail_handles(app: &mut AppModel) {
    app.thumbnail_handles.retain(|key, _| {
        app.history.iter().any(|item| match &item.entry {
            ClipboardEntry::Image { bytes, hash, .. } => key == &(*hash, bytes.len()),
            ClipboardEntry::Text(_) => false,
        })
    });
}

fn parse_usize_field(input: &str) -> Result<usize, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("value is required");
    }
    trimmed
        .parse::<usize>()
        .map_err(|_| "must be a valid positive integer")
}

fn parse_u32_field(input: &str) -> Result<u32, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("value is required");
    }
    trimmed
        .parse::<u32>()
        .map_err(|_| "must be a valid positive integer")
}

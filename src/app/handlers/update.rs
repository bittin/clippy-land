use super::{history, scroll};
use crate::app::model::{FocusPart, HistoryItem};
use crate::app::{AppModel, Message};
use crate::services::clipboard::ClipboardEntry;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
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

            let pinned = app
                .history
                .iter()
                .position(|it| &it.entry == &entry)
                .and_then(|idx| app.history.remove(idx))
                .is_some_and(|it| it.pinned);

            history::insert_after_pins(&mut app.history, HistoryItem { entry, pinned });
            history::trim_history(&mut app.history);
        }
        Message::TogglePin(index) => {
            history::toggle_pin(&mut app.history, index);
        }
        Message::CopyFromHistory(index) => {
            if let Some(item) = app.history.get(index) {
                history::copy_history_item(item);
            }
        }
        Message::RemoveHistory(index) => {
            let _ = app.history.remove(index);
        }
        Message::ClearHistory => {
            app.history.clear();
        }
        Message::HoverEntry(opt) => {
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
            if app.history.is_empty() {
                return Task::none();
            }
            let len = app.history.len();
            let new_idx = match app.hovered_index {
                Some(idx) => {
                    if idx == 0 {
                        len - 1
                    } else {
                        idx - 1
                    }
                }
                None => len - 1,
            };
            app.hovered_index = Some(new_idx);
            app.hovered_focus = None;
            app.keyboard_focus = Some((new_idx, FocusPart::Entry));
            app.at_scroll_bottom = false;
            return scroll::scroll_selection_into_view(app, new_idx);
        }
        Message::MoveSelectionDown => {
            if app.history.is_empty() {
                return Task::none();
            }
            let len = app.history.len();
            let new_idx = match app.hovered_index {
                Some(idx) => (idx + 1) % len,
                None => 0,
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
                        history::toggle_pin(&mut app.history, idx);
                    }
                    FocusPart::Remove => {
                        let _ = app.history.remove(idx);
                    }
                }
            } else if let Some(idx) = app.hovered_index {
                if let Some(item) = app.history.get(idx) {
                    history::copy_history_item(item);
                }
            }
        }
        Message::TogglePopup => {
            return if let Some(p) = app.popup.take() {
                destroy_popup(p)
            } else {
                let new_id = cosmic::iced::window::Id::unique();
                app.popup.replace(new_id);
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
        Message::PopupClosed(id) => {
            if app.popup.as_ref() == Some(&id) {
                app.popup = None;
                app.hovered_index = None;
                app.at_scroll_bottom = false;
                app.history_viewport = None;
            }
        }
    }
    Task::none()
}

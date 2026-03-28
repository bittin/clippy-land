use super::model::HistoryItem;
use super::{AppModel, Message};
use crate::services::clipboard;
use cosmic::iced::Subscription;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::iced::futures::channel::mpsc;
use futures_util::SinkExt;
use std::collections::VecDeque;
use std::time::Duration;

const MAX_HISTORY: usize = 30;
const MAX_PINNED: usize = 5;

pub fn subscription(_app: &AppModel) -> Subscription<Message> {
    struct ClipboardSubscription;

    Subscription::batch(vec![Subscription::run_with(
        std::any::TypeId::of::<ClipboardSubscription>(),
        |_| {
            cosmic::iced::stream::channel(1, move |mut channel: mpsc::Sender<Message>| async move {
                let mut last_seen: Option<clipboard::ClipboardFingerprint> = None;

                loop {
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let next = tokio::task::spawn_blocking(clipboard::read_clipboard_entry)
                        .await
                        .ok()
                        .flatten();

                    let Some(next) = next else {
                        continue;
                    };

                    let next_fp = next.fingerprint();
                    if last_seen.as_ref() == Some(&next_fp) {
                        continue;
                    }

                    last_seen = Some(next_fp);

                    if channel.send(Message::ClipboardChanged(next)).await.is_err() {
                        break;
                    }
                }
            })
        },
    )])
}

fn pinned_count(history: &VecDeque<HistoryItem>) -> usize {
    history.iter().filter(|it| it.pinned).count()
}

fn insert_after_pins(history: &mut VecDeque<HistoryItem>, item: HistoryItem) {
    let pos = history.iter().take_while(|it| it.pinned).count();
    history.insert(pos, item);
}

fn trim_history(history: &mut VecDeque<HistoryItem>) {
    while history.len() > MAX_HISTORY {
        if let Some(idx) = history.iter().rposition(|it| !it.pinned) {
            let _ = history.remove(idx);
        } else {
            break;
        }
    }
}

pub fn update(app: &mut AppModel, message: Message) -> Task<cosmic::Action<Message>> {
    match message {
        Message::ClipboardChanged(entry) => {
            if app
                .history
                .front()
                .is_some_and(|it: &HistoryItem| &it.entry == &entry)
            {
                return Task::none();
            }

            if let clipboard::ClipboardEntry::Text(text) = &entry {
                if should_ignore_clipboard_entry(text) {
                    return Task::none();
                }
            }

            // Remove any existing entries that match to keep the history unique, but keep pin state.
            let pinned = app
                .history
                .iter()
                .position(|it| &it.entry == &entry)
                .and_then(|idx| app.history.remove(idx))
                .is_some_and(|it| it.pinned);

            insert_after_pins(&mut app.history, HistoryItem { entry, pinned });
            trim_history(&mut app.history);
        }
        Message::TogglePin(index) => {
            let Some(mut item) = app.history.remove(index) else {
                return Task::none();
            };

            if item.pinned {
                item.pinned = false;
                insert_after_pins(&mut app.history, item);
            } else if pinned_count(&app.history) >= MAX_PINNED {
                // Pin limit reached; keep the item where it was.
                app.history.insert(index, item);
            } else {
                item.pinned = true;
                insert_after_pins(&mut app.history, item);
            }
        }
        Message::CopyFromHistory(index) => {
            if let Some(item) = app.history.get(index) {
                match &item.entry {
                    clipboard::ClipboardEntry::Text(text) => {
                        _ = clipboard::write_clipboard_text(text);
                    }
                    clipboard::ClipboardEntry::Image { mime, bytes, .. } => {
                        _ = clipboard::write_clipboard_image(mime, bytes);
                    }
                }
            }
            if let Some(p) = app.popup.take() {
                app.hovered_index = None;
                app.at_scroll_bottom = false;
                return destroy_popup(p);
            }
        }
        Message::RemoveHistory(index) => {
            let _ = app.history.remove(index);
        }
        Message::ClearHistory => {
            app.history.clear();
        }
        Message::HoverEntry(opt) => {
            app.hovered_index = opt;
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
            }
        }
    }
    Task::none()
}

fn should_ignore_clipboard_entry(entry: &str) -> bool {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return true;
    }

    if trimmed.chars().all(|c| {
        c.is_ascii_digit() || matches!(c, ',' | '.' | ':' | ';' | '/' | '\\' | '_' | '-' | ' ')
    }) && trimmed.chars().count() <= 8
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_entry(text: &str) -> clipboard::ClipboardEntry {
        clipboard::ClipboardEntry::Text(text.to_string())
    }

    fn text_item(text: &str, pinned: bool) -> HistoryItem {
        HistoryItem {
            entry: text_entry(text),
            pinned,
        }
    }

    fn item_text(item: &HistoryItem) -> &str {
        match &item.entry {
            clipboard::ClipboardEntry::Text(text) => text,
            clipboard::ClipboardEntry::Image { .. } => {
                panic!("expected text entry in handler tests")
            }
        }
    }

    #[test]
    fn ignores_empty_and_short_numericish_entries() {
        assert!(should_ignore_clipboard_entry(""));
        assert!(should_ignore_clipboard_entry("  \n\t  "));
        assert!(should_ignore_clipboard_entry("12-34"));
        assert!(should_ignore_clipboard_entry("1,2,3"));
    }

    #[test]
    fn keeps_nontrivial_entries() {
        assert!(!should_ignore_clipboard_entry("123456789"));
        assert!(!should_ignore_clipboard_entry("abc123"));
        assert!(!should_ignore_clipboard_entry("42 is the answer"));
    }

    #[test]
    fn clipboard_changed_dedupes_and_preserves_pin_state() {
        let repeated = text_entry("repeat");
        let mut app = AppModel::default();
        app.history.push_back(text_item("front", false));
        app.history.push_back(HistoryItem {
            entry: repeated.clone(),
            pinned: true,
        });
        app.history.push_back(text_item("tail", false));

        let _ = update(&mut app, Message::ClipboardChanged(repeated.clone()));

        let matches = app.history.iter().filter(|it| it.entry == repeated).count();
        assert_eq!(matches, 1);

        let idx = app
            .history
            .iter()
            .position(|it| it.entry == repeated)
            .expect("entry should still exist");
        assert!(app.history[idx].pinned);
    }

    #[test]
    fn toggling_pinned_item_moves_it_after_pinned_section() {
        let mut app = AppModel::default();
        app.history.push_back(text_item("a", true));
        app.history.push_back(text_item("b", true));
        app.history.push_back(text_item("c", false));

        let _ = update(&mut app, Message::TogglePin(0));

        assert!(app.history[0].pinned);
        assert_eq!(item_text(&app.history[0]), "b");
        assert!(!app.history[1].pinned);
        assert_eq!(item_text(&app.history[1]), "a");
    }

    #[test]
    fn toggle_pin_respects_max_pinned_limit() {
        let mut app = AppModel::default();
        for i in 0..MAX_PINNED {
            app.history.push_back(text_item(&format!("pin-{i}"), true));
        }
        app.history.push_back(text_item("unpinned", false));

        let _ = update(&mut app, Message::TogglePin(MAX_PINNED));

        assert_eq!(pinned_count(&app.history), MAX_PINNED);
        assert_eq!(item_text(&app.history[MAX_PINNED]), "unpinned");
        assert!(!app.history[MAX_PINNED].pinned);
    }

    #[test]
    fn clipboard_changed_trims_to_max_history() {
        let mut app = AppModel::default();
        for i in 0..MAX_HISTORY {
            app.history
                .push_back(text_item(&format!("item-{i}"), false));
        }

        let _ = update(
            &mut app,
            Message::ClipboardChanged(text_entry("fresh-entry")),
        );

        assert_eq!(app.history.len(), MAX_HISTORY);
        assert_eq!(
            item_text(app.history.front().expect("front entry exists")),
            "fresh-entry"
        );
        assert!(!app.history.iter().any(|it| item_text(it) == "item-29"));
    }

    #[test]
    fn clear_history_removes_all_entries() {
        let mut app = AppModel::default();
        app.history.push_back(text_item("pinned", true));
        app.history.push_back(text_item("regular", false));

        let _ = update(&mut app, Message::ClearHistory);

        assert!(app.history.is_empty());
    }

    #[test]
    fn clear_history_is_safe_for_empty_history() {
        let mut app = AppModel::default();

        let _ = update(&mut app, Message::ClearHistory);

        assert!(app.history.is_empty());
    }

    #[test]
    fn selecting_entry_closes_popup() {
        let mut app = AppModel::default();
        app.popup = Some(cosmic::iced::window::Id::unique());
        app.history.push_back(text_item("copy me", false));

        let _ = update(&mut app, Message::CopyFromHistory(0));

        assert!(app.popup.is_none());
    }
}

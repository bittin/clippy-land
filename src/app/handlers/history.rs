use crate::app::model::HistoryItem;
use crate::services::clipboard::{self, ClipboardEntry};
use crate::settings::AppSettings;
use std::collections::VecDeque;

pub(super) fn pinned_count(history: &VecDeque<HistoryItem>) -> usize {
    history.iter().filter(|it| it.pinned).count()
}

pub(super) fn insert_after_pins(history: &mut VecDeque<HistoryItem>, item: HistoryItem) {
    let pos = history.iter().take_while(|it| it.pinned).count();
    history.insert(pos, item);
}

fn reorder_pins_first(history: &mut VecDeque<HistoryItem>) {
    let pinned: Vec<_> = history.iter().filter(|it| it.pinned).cloned().collect();
    let unpinned: Vec<_> = history.iter().filter(|it| !it.pinned).cloned().collect();

    history.clear();
    history.extend(pinned);
    history.extend(unpinned);
}

pub(super) fn reconcile_limits(history: &mut VecDeque<HistoryItem>, settings: &AppSettings) {
    let max_history = settings.max_history.max(1);
    let max_pinned = settings.max_pinned.min(max_history);

    let mut pinned_seen = 0usize;
    for item in history.iter_mut() {
        if item.pinned {
            if pinned_seen < max_pinned {
                pinned_seen += 1;
            } else {
                item.pinned = false;
            }
        }
    }

    reorder_pins_first(history);

    while history.len() > max_history {
        if let Some(idx) = history.iter().rposition(|it| !it.pinned) {
            let _ = history.remove(idx);
        } else {
            let _ = history.pop_back();
        }
    }
}

pub(super) fn trim_history(history: &mut VecDeque<HistoryItem>, settings: &AppSettings) {
    reconcile_limits(history, settings);
}

pub(super) fn toggle_pin(
    history: &mut VecDeque<HistoryItem>,
    index: usize,
    settings: &AppSettings,
) -> bool {
    let Some(mut item) = history.remove(index) else {
        return false;
    };

    let max_pinned = settings.max_pinned.min(settings.max_history);

    if item.pinned {
        item.pinned = false;
        insert_after_pins(history, item);
    } else if pinned_count(history) >= max_pinned {
        history.insert(index, item);
        return false;
    } else {
        item.pinned = true;
        insert_after_pins(history, item);
    }

    reconcile_limits(history, settings);
    true
}

pub(super) fn copy_history_item(item: &HistoryItem) {
    copy_clipboard_entry(&item.entry);
}

pub(super) fn copy_clipboard_entry(entry: &ClipboardEntry) {
    match entry {
        ClipboardEntry::Text(text) => {
            _ = clipboard::write_clipboard_text(text);
        }
        ClipboardEntry::Image { mime, bytes, .. } => {
            _ = clipboard::write_clipboard_image(mime, bytes);
        }
    }
}

pub(super) fn should_ignore_clipboard_entry(entry: &str) -> bool {
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

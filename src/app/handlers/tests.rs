use super::{history, scroll, update};
use crate::app::model::{FocusPart, HistoryItem};
use crate::app::view;
use crate::app::{AppModel, Message};
use crate::services::clipboard;
use crate::settings::AppSettings;

fn text_entry(text: &str) -> clipboard::ClipboardEntry {
    clipboard::ClipboardEntry::Text(text.to_string())
}

fn text_item(text: &str, pinned: bool) -> HistoryItem {
    HistoryItem {
        entry: text_entry(text),
        pinned,
    }
}

fn image_entry(hash: u64) -> clipboard::ClipboardEntry {
    clipboard::ClipboardEntry::Image {
        mime: "image/png".to_string(),
        bytes: vec![1, 2, 3, 4],
        hash,
        thumbnail_png: Some(vec![137, 80, 78, 71]),
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

fn test_settings(max_history: usize, max_pinned: usize) -> AppSettings {
    AppSettings {
        max_history,
        max_pinned,
        ..AppSettings::default()
    }
    .normalized()
}

#[test]
fn ignores_empty_and_short_numericish_entries() {
    assert!(history::should_ignore_clipboard_entry(""));
    assert!(history::should_ignore_clipboard_entry("  \n\t  "));
    assert!(history::should_ignore_clipboard_entry("12-34"));
    assert!(history::should_ignore_clipboard_entry("1,2,3"));
}

#[test]
fn keeps_nontrivial_entries() {
    assert!(!history::should_ignore_clipboard_entry("123456789"));
    assert!(!history::should_ignore_clipboard_entry("abc123"));
    assert!(!history::should_ignore_clipboard_entry("42 is the answer"));
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
fn clipboard_changed_caches_and_prunes_thumbnail_handles() {
    let mut app = AppModel::default();

    let _ = update(&mut app, Message::ClipboardChanged(image_entry(42)));
    assert_eq!(app.thumbnail_handles.len(), 1);

    let _ = update(&mut app, Message::RemoveHistory(0));
    assert!(app.history.is_empty());
    assert!(app.thumbnail_handles.is_empty());
}

#[test]
fn clipboard_changed_recomputes_filtered_indices_cache() {
    let mut app = AppModel::default();
    app.search_query = "ap".into();
    app.recompute_filtered_indices();
    assert!(app.filtered_indices.is_empty());

    let _ = update(
        &mut app,
        Message::ClipboardChanged(clipboard::ClipboardEntry::Text("apple".into())),
    );

    assert_eq!(app.filtered_indices, vec![0]);
}

#[test]
fn clear_history_clears_thumbnail_handles() {
    let mut app = AppModel::default();

    let _ = update(&mut app, Message::ClipboardChanged(image_entry(7)));
    assert_eq!(app.thumbnail_handles.len(), 1);

    let _ = update(&mut app, Message::ClearHistory);
    assert!(app.history.is_empty());
    assert!(app.thumbnail_handles.is_empty());
}

#[test]
fn open_text_overlay_sets_overlay_index_for_text_entry() {
    let mut app = AppModel::default();
    app.history
        .push_back(text_item("first line\nsecond line", false));

    let _ = update(&mut app, Message::OpenTextOverlay(0));

    assert_eq!(app.text_overlay_index, Some(0));
}

#[test]
fn open_text_overlay_ignores_image_entries() {
    let mut app = AppModel::default();
    app.history.push_back(HistoryItem {
        entry: image_entry(9),
        pinned: false,
    });

    let _ = update(&mut app, Message::OpenTextOverlay(0));

    assert!(app.text_overlay_index.is_none());
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
    app.settings = test_settings(30, 5);

    for i in 0..app.settings.max_pinned {
        app.history.push_back(text_item(&format!("pin-{i}"), true));
    }
    app.history.push_back(text_item("unpinned", false));

    let max_pinned = app.settings.max_pinned;
    let _ = update(&mut app, Message::TogglePin(max_pinned));

    assert_eq!(history::pinned_count(&app.history), app.settings.max_pinned);
    assert_eq!(item_text(&app.history[app.settings.max_pinned]), "unpinned");
    assert!(!app.history[app.settings.max_pinned].pinned);
}

#[test]
fn clipboard_changed_trims_to_max_history() {
    let mut app = AppModel::default();
    app.settings = test_settings(30, 5);

    for i in 0..app.settings.max_history {
        app.history
            .push_back(text_item(&format!("item-{i}"), false));
    }

    let _ = update(
        &mut app,
        Message::ClipboardChanged(text_entry("fresh-entry")),
    );

    assert_eq!(app.history.len(), app.settings.max_history);
    assert_eq!(
        item_text(app.history.front().expect("front entry exists")),
        "fresh-entry"
    );
    assert!(!app.history.iter().any(|it| item_text(it) == "item-29"));
}

#[test]
fn reconcile_limits_unpins_overflow_then_reorders() {
    let mut history_vec = std::collections::VecDeque::new();
    history_vec.push_back(text_item("a", true));
    history_vec.push_back(text_item("b", true));
    history_vec.push_back(text_item("c", true));
    history_vec.push_back(text_item("d", false));

    let settings = test_settings(10, 2);
    history::reconcile_limits(&mut history_vec, &settings);

    assert!(history_vec[0].pinned);
    assert!(history_vec[1].pinned);
    assert!(!history_vec[2].pinned);
    assert_eq!(item_text(&history_vec[0]), "a");
    assert_eq!(item_text(&history_vec[1]), "b");
    assert_eq!(item_text(&history_vec[2]), "c");
}

#[test]
fn reconcile_limits_trims_oldest_unpinned_first() {
    let mut history_vec = std::collections::VecDeque::new();
    history_vec.push_back(text_item("p0", true));
    for i in 0..30 {
        history_vec.push_back(text_item(&format!("u{i}"), false));
    }

    let settings = test_settings(30, 1);
    history::reconcile_limits(&mut history_vec, &settings);

    assert_eq!(history_vec.len(), 30);
    assert_eq!(item_text(&history_vec[0]), "p0");
    assert_eq!(item_text(&history_vec[1]), "u0");
    assert_eq!(
        item_text(history_vec.back().expect("last entry exists")),
        "u28"
    );
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
fn desired_scroll_y_moves_selection_into_visible_window() {
    let offset = scroll::desired_scroll_y(Some(0.4), Some(0.25), 19, 30);

    assert!(matches!(offset, Some(value) if (value - 0.7).abs() < 0.000_1));
}

#[test]
fn desired_scroll_y_skips_when_selection_is_already_centered() {
    let offset = scroll::desired_scroll_y(Some(0.42857143), Some(0.3), 13, 30);

    assert_eq!(offset, None);
}

#[test]
fn desired_scroll_y_falls_back_to_target_ratio_without_viewport() {
    let offset = scroll::desired_scroll_y(None, None, 5, 10);

    assert_eq!(offset, Some(0.55));
}

// ── SearchChanged handler ────────────────────────────────────────────────────

#[test]
fn search_changed_updates_query_and_clears_hover_and_keyboard() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("hello", false));
    app.hovered_index = Some(0);
    app.hovered_focus = Some((0, FocusPart::Entry));
    app.keyboard_focus = Some((0, FocusPart::Pin));

    let _ = update(&mut app, Message::SearchChanged("he".into()));

    assert_eq!(app.search_query, "he");
    assert!(app.hovered_index.is_none());
    assert!(app.hovered_focus.is_none());
    assert!(app.keyboard_focus.is_none());
}

#[test]
fn search_changed_empty_string_clears_query() {
    let mut app = AppModel::default();
    app.search_query = "old".into();

    let _ = update(&mut app, Message::SearchChanged(String::new()));

    assert!(app.search_query.is_empty());
}

// ── RemoveHistory ────────────────────────────────────────────────────────────

#[test]
fn remove_history_removes_entry_at_index() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("first", false));
    app.history.push_back(text_item("second", false));
    app.history.push_back(text_item("third", false));

    let _ = update(&mut app, Message::RemoveHistory(1));

    assert_eq!(app.history.len(), 2);
    assert_eq!(item_text(&app.history[0]), "first");
    assert_eq!(item_text(&app.history[1]), "third");
}

#[test]
fn remove_history_last_item_leaves_empty() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("only", false));

    let _ = update(&mut app, Message::RemoveHistory(0));

    assert!(app.history.is_empty());
}

// ── HoverEntry ───────────────────────────────────────────────────────────────

#[test]
fn hover_entry_sets_hover_state_and_clears_keyboard_focus() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("item", false));
    app.keyboard_focus = Some((0, FocusPart::Entry));

    let _ = update(&mut app, Message::HoverEntry(Some((0, FocusPart::Pin))));

    assert_eq!(app.hovered_index, Some(0));
    assert_eq!(app.hovered_focus, Some((0, FocusPart::Pin)));
    assert!(app.keyboard_focus.is_none());
}

#[test]
fn hover_entry_none_clears_hover_state() {
    let mut app = AppModel::default();
    app.hovered_index = Some(2);
    app.hovered_focus = Some((2, FocusPart::Remove));

    let _ = update(&mut app, Message::HoverEntry(None));

    assert!(app.hovered_index.is_none());
    assert!(app.hovered_focus.is_none());
}

#[test]
fn redundant_hover_entry_does_not_clear_keyboard_focus() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("item", false));
    app.hovered_index = Some(0);
    app.hovered_focus = Some((0, FocusPart::Entry));
    app.keyboard_focus = Some((0, FocusPart::Remove));

    let _ = update(&mut app, Message::HoverEntry(Some((0, FocusPart::Entry))));

    assert_eq!(app.hovered_index, Some(0));
    assert_eq!(app.hovered_focus, Some((0, FocusPart::Entry)));
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Remove)));
}

#[test]
fn hover_entry_action_exit_can_fall_back_to_entry_without_clearing() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("item", false));

    let _ = update(&mut app, Message::HoverEntry(Some((0, FocusPart::Pin))));
    assert_eq!(app.hovered_focus, Some((0, FocusPart::Pin)));

    let _ = update(&mut app, Message::HoverEntry(Some((0, FocusPart::Entry))));
    assert_eq!(app.hovered_index, Some(0));
    assert_eq!(app.hovered_focus, Some((0, FocusPart::Entry)));
}

// ── Keyboard nav with filtered results ───────────────────────────────────────

#[test]
fn move_selection_down_steps_through_filtered_indices() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("apple", false)); // idx 0 – matches
    app.history.push_back(text_item("banana", false)); // idx 1 – no match
    app.history.push_back(text_item("apricot", false)); // idx 2 – matches
    app.search_query = "ap".into();
    app.recompute_filtered_indices();

    // First press: no previous selection → picks first filtered idx (0)
    let _ = update(&mut app, Message::MoveSelectionDown);
    assert_eq!(app.hovered_index, Some(0));

    // Second press: from idx 0 → next in [0,2] is idx 2
    let _ = update(&mut app, Message::MoveSelectionDown);
    assert_eq!(app.hovered_index, Some(2));

    // Third press: wraps back to first (idx 0)
    let _ = update(&mut app, Message::MoveSelectionDown);
    assert_eq!(app.hovered_index, Some(0));
}

#[test]
fn move_selection_up_wraps_to_last_filtered_index() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("apple", false)); // idx 0
    app.history.push_back(text_item("banana", false)); // idx 1
    app.history.push_back(text_item("apricot", false)); // idx 2
    app.search_query = "ap".into();
    app.recompute_filtered_indices();

    // No selection → Up picks last filtered (idx 2)
    let _ = update(&mut app, Message::MoveSelectionUp);
    assert_eq!(app.hovered_index, Some(2));

    // Again: from idx 2 → prev in [0,2] is idx 0
    let _ = update(&mut app, Message::MoveSelectionUp);
    assert_eq!(app.hovered_index, Some(0));
}

#[test]
fn toggle_settings_panel_opens_and_prefills_draft() {
    let mut app = AppModel::default();
    app.settings = AppSettings {
        max_history: 444,
        max_pinned: 33,
        max_image_bytes: 3 * 1024 * 1024,
        max_image_dimension_px: 2048,
        ..AppSettings::default()
    }
    .normalized();

    let _ = update(&mut app, Message::ToggleSettingsPanel);

    assert!(app.settings_open);
    assert_eq!(app.settings_draft.max_history, "444");
    assert_eq!(app.settings_draft.max_pinned, "33");
    assert_eq!(
        app.settings_draft.max_image_bytes,
        (3 * 1024 * 1024).to_string()
    );
    assert_eq!(app.settings_draft.max_image_dimension_px, "2048");
}

#[test]
fn apply_settings_rejects_invalid_input_with_error() {
    let mut app = AppModel::default();
    app.settings_open = true;
    app.settings_draft.max_history = "not-a-number".into();
    app.settings_draft.max_pinned = "10".into();
    app.settings_draft.max_image_bytes = "1048576".into();
    app.settings_draft.max_image_dimension_px = "2048".into();

    let _ = update(&mut app, Message::ApplySettings);

    assert!(app.settings_open);
    assert!(app.settings_error.is_some());
}

#[test]
fn apply_settings_rejects_out_of_range_values() {
    let mut app = AppModel::default();
    app.settings_open = true;
    app.settings_draft.max_history = "1".into();
    app.settings_draft.max_pinned = "0".into();
    app.settings_draft.max_image_bytes = "1048576".into();
    app.settings_draft.max_image_dimension_px = "1024".into();

    let _ = update(&mut app, Message::ApplySettings);

    assert!(app.settings_open);
    let err = app.settings_error.expect("range error should be present");
    assert!(err.contains("Max history must be between"));
}

#[test]
fn apply_settings_rejects_pinned_greater_than_history() {
    let mut app = AppModel::default();
    app.settings_open = true;
    app.settings_draft.max_history = "100".into();
    app.settings_draft.max_pinned = "101".into();
    app.settings_draft.max_image_bytes = "1048576".into();
    app.settings_draft.max_image_dimension_px = "1024".into();

    let _ = update(&mut app, Message::ApplySettings);

    assert!(app.settings_open);
    assert_eq!(
        app.settings_error.as_deref(),
        Some("Max pinned cannot be greater than max history")
    );
}

#[test]
fn apply_settings_rejects_image_bytes_below_minimum() {
    let mut app = AppModel::default();
    app.settings_open = true;
    app.settings_draft.max_history = "200".into();
    app.settings_draft.max_pinned = "20".into();
    app.settings_draft.max_image_bytes = "1".into();
    app.settings_draft.max_image_dimension_px = "1024".into();

    let _ = update(&mut app, Message::ApplySettings);

    assert!(app.settings_open);
    let err = app.settings_error.expect("range error should be present");
    assert!(err.contains("Max image bytes must be between"));
}

#[test]
fn apply_settings_updates_runtime_settings_and_closes_panel() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should be after unix epoch")
        .as_nanos();
    let cfg_path = std::env::temp_dir().join(format!("clippy-land-test-settings-{unique}.toml"));
    unsafe { std::env::set_var("CLIPPY_LAND_CONFIG", &cfg_path) };

    let mut app = AppModel::default();
    app.settings_open = true;
    app.settings_draft.max_history = "350".into();
    app.settings_draft.max_pinned = "30".into();
    app.settings_draft.max_image_bytes = "2097152".into();
    app.settings_draft.max_image_dimension_px = "4096".into();

    let _ = update(&mut app, Message::ApplySettings);

    assert!(!app.settings_open);
    assert!(app.settings_error.is_none());
    assert_eq!(app.settings.max_history, 350);
    assert_eq!(app.settings.max_pinned, 30);
    assert_eq!(app.settings.max_image_bytes, 2 * 1024 * 1024);
    assert_eq!(app.settings.max_image_dimension_px, 4096);

    let persisted = std::fs::read_to_string(&cfg_path).expect("settings should be written");
    assert!(persisted.contains("max_history = 350"));
    assert!(persisted.contains("max_pinned = 30"));

    let _ = std::fs::remove_file(cfg_path);
    unsafe { std::env::remove_var("CLIPPY_LAND_CONFIG") };
}

#[test]
fn move_selection_does_nothing_when_filtered_list_is_empty() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("apple", false));
    app.search_query = "zzz".into();
    app.recompute_filtered_indices();

    let _ = update(&mut app, Message::MoveSelectionDown);
    assert!(app.hovered_index.is_none());

    let _ = update(&mut app, Message::MoveSelectionUp);
    assert!(app.hovered_index.is_none());
}

#[test]
fn search_changed_recomputes_filtered_indices_cache() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("apple", false));
    app.history.push_back(text_item("banana", false));
    app.recompute_filtered_indices();
    assert_eq!(app.filtered_indices, vec![0, 1]);

    let _ = update(&mut app, Message::SearchChanged("ap".into()));
    assert_eq!(app.filtered_indices, vec![0]);
}

#[test]
fn search_changed_closes_text_overlay() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("overlay text", false));
    app.text_overlay_index = Some(0);

    let _ = update(&mut app, Message::SearchChanged("over".into()));

    assert!(app.text_overlay_index.is_none());
}

#[test]
fn close_text_overlay_message_clears_overlay_index() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("overlay text", false));
    app.text_overlay_index = Some(0);

    let _ = update(&mut app, Message::CloseTextOverlay);

    assert!(app.text_overlay_index.is_none());
}

#[test]
fn close_text_overlay_message_is_noop_when_overlay_not_open() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("overlay text", false));

    let _ = update(&mut app, Message::CloseTextOverlay);

    assert!(app.text_overlay_index.is_none());
}

#[test]
fn escape_pressed_closes_overlay_without_closing_popup() {
    let mut app = AppModel::default();
    let popup_id = cosmic::iced::window::Id::unique();
    app.popup = Some(popup_id);
    app.history
        .push_back(text_item("first line\nsecond line", false));
    app.text_overlay_index = Some(0);

    let _ = update(&mut app, Message::EscapePressed);

    assert_eq!(app.popup, Some(popup_id));
    assert!(app.text_overlay_index.is_none());
}

#[test]
fn escape_pressed_closes_popup_when_overlay_not_open() {
    let mut app = AppModel::default();
    let popup_id = cosmic::iced::window::Id::unique();
    app.popup = Some(popup_id);
    app.popup_is_layer_surface = true;

    let _ = update(&mut app, Message::EscapePressed);

    assert!(app.popup.is_none());
}

// ── MoveFocusLeft / MoveFocusRight ───────────────────────────────────────────

#[test]
fn move_focus_right_cycles_entry_pin_remove() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("item", false));
    app.hovered_index = Some(0);
    app.keyboard_focus = Some((0, FocusPart::Entry));

    let _ = update(&mut app, Message::MoveFocusRight);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Pin)));

    let _ = update(&mut app, Message::MoveFocusRight);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Remove)));

    let _ = update(&mut app, Message::MoveFocusRight);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Entry)));
}

#[test]
fn move_focus_right_cycles_entry_preview_pin_remove_when_preview_available() {
    let mut app = AppModel::default();
    app.history
        .push_back(text_item("first line\nsecond line", false));
    app.hovered_index = Some(0);
    app.keyboard_focus = Some((0, FocusPart::Entry));

    let _ = update(&mut app, Message::MoveFocusRight);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Preview)));

    let _ = update(&mut app, Message::MoveFocusRight);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Pin)));

    let _ = update(&mut app, Message::MoveFocusRight);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Remove)));

    let _ = update(&mut app, Message::MoveFocusRight);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Entry)));
}

#[test]
fn move_focus_left_cycles_entry_remove_pin() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("item", false));
    app.hovered_index = Some(0);
    app.keyboard_focus = Some((0, FocusPart::Entry));

    let _ = update(&mut app, Message::MoveFocusLeft);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Remove)));

    let _ = update(&mut app, Message::MoveFocusLeft);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Pin)));

    let _ = update(&mut app, Message::MoveFocusLeft);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Entry)));
}

#[test]
fn move_focus_left_cycles_entry_remove_pin_preview_when_preview_available() {
    let mut app = AppModel::default();
    app.history
        .push_back(text_item("first line\nsecond line", false));
    app.hovered_index = Some(0);
    app.keyboard_focus = Some((0, FocusPart::Entry));

    let _ = update(&mut app, Message::MoveFocusLeft);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Remove)));

    let _ = update(&mut app, Message::MoveFocusLeft);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Pin)));

    let _ = update(&mut app, Message::MoveFocusLeft);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Preview)));

    let _ = update(&mut app, Message::MoveFocusLeft);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Entry)));
}

#[test]
fn activate_selection_opens_text_overlay_when_preview_is_focused() {
    let mut app = AppModel::default();
    app.history
        .push_back(text_item("first line\nsecond line", false));
    app.keyboard_focus = Some((0, FocusPart::Preview));

    let _ = update(&mut app, Message::ActivateSelection);

    assert_eq!(app.text_overlay_index, Some(0));
}

#[test]
fn move_focus_without_hover_initialises_to_entry() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("item", false));
    app.hovered_index = Some(0);
    // keyboard_focus starts as None

    let _ = update(&mut app, Message::MoveFocusRight);
    assert_eq!(app.keyboard_focus, Some((0, FocusPart::Entry)));
}

// ── PopupClosed ──────────────────────────────────────────────────────────────

#[test]
fn popup_closed_clears_popup_and_search() {
    let mut app = AppModel::default();
    let id = cosmic::iced::window::Id::unique();
    app.popup = Some(id);
    app.popup_is_layer_surface = true;
    app.search_query = "hello".into();
    app.hovered_index = Some(1);
    app.at_scroll_bottom = true;

    let _ = update(&mut app, Message::PopupClosed(id));

    assert!(app.popup.is_none());
    assert!(!app.popup_is_layer_surface);
    assert!(app.search_query.is_empty());
    assert!(app.hovered_index.is_none());
    assert!(!app.at_scroll_bottom);
}

#[test]
fn popup_closed_ignores_mismatched_id() {
    let mut app = AppModel::default();
    let real_id = cosmic::iced::window::Id::unique();
    let other_id = cosmic::iced::window::Id::unique();
    app.popup = Some(real_id);
    app.search_query = "query".into();

    let _ = update(&mut app, Message::PopupClosed(other_id));

    assert_eq!(app.popup, Some(real_id));
    assert_eq!(app.search_query, "query");
}

#[test]
fn popup_open_trace_starts_on_open_and_clears_after_first_view() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("row", false));
    app.recompute_filtered_indices();
    let popup_id = cosmic::iced::window::Id::unique();
    app.popup = Some(popup_id);

    app.begin_popup_open_trace("test");
    assert!(app.popup_open_trace_pending_for_test());

    let _ = view::view_window(&app, popup_id);
    assert!(app.popup_open_trace_pending_for_test());

    let _ = update(&mut app, Message::PopupRedraw(popup_id));
    assert!(!app.popup_open_trace_pending_for_test());
}

#[test]
fn popup_closed_cancels_pending_popup_open_trace() {
    let mut app = AppModel::default();
    app.history.push_back(text_item("row", false));
    app.recompute_filtered_indices();
    let popup_id = cosmic::iced::window::Id::unique();
    app.popup = Some(popup_id);

    app.begin_popup_open_trace("test");
    assert!(app.popup_open_trace_pending_for_test());

    let _ = update(&mut app, Message::PopupClosed(popup_id));
    assert!(!app.popup_open_trace_pending_for_test());
}

// ── WindowUnfocused ──────────────────────────────────────────────────────────

#[test]
fn window_unfocused_only_closes_layer_surface_popups() {
    let mut app = AppModel::default();
    let id = cosmic::iced::window::Id::unique();
    app.popup = Some(id);
    app.popup_is_layer_surface = false; // XDG popup, should NOT be closed
    app.search_query = "query".into();

    let _ = update(&mut app, Message::WindowUnfocused(id));

    // Popup should remain open for non-layer-surface popups
    assert!(app.popup.is_some());
    assert_eq!(app.search_query, "query");
}

// ── IPC signal file path ─────────────────────────────────────────────────────

#[test]
fn ipc_signal_path_is_none_when_env_var_unset() {
    unsafe { std::env::remove_var("CLIPPY_LAND_SIGNAL_FILE") };

    // Temporarily remove XDG_RUNTIME_DIR if present
    let saved = std::env::var("XDG_RUNTIME_DIR").ok();
    unsafe { std::env::remove_var("XDG_RUNTIME_DIR") };

    let result = crate::ipc::get_signal_file_path();
    assert!(result.is_none());

    // Restore
    if let Some(val) = saved {
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", val) };
    }
}

#[test]
fn ipc_signal_path_appends_filename_to_runtime_dir() {
    unsafe { std::env::remove_var("CLIPPY_LAND_SIGNAL_FILE") };
    unsafe { std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-runtime") };

    let result = crate::ipc::get_signal_file_path();

    // Restore – don't leave test pollution
    unsafe { std::env::remove_var("XDG_RUNTIME_DIR") };

    let path = result.expect("should return a path");
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some("clippy-land-toggle")
    );
    assert!(path.to_string_lossy().starts_with("/tmp/test-runtime"));
}

#[test]
fn ipc_signal_path_prefers_override_env_var() {
    unsafe {
        std::env::set_var("CLIPPY_LAND_SIGNAL_FILE", "/tmp/clippy-land-test-signal");
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-runtime-ignored");
    }

    let result = crate::ipc::get_signal_file_path();

    unsafe {
        std::env::remove_var("CLIPPY_LAND_SIGNAL_FILE");
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    let path = result.expect("override signal path should be returned");
    assert_eq!(
        path,
        std::path::PathBuf::from("/tmp/clippy-land-test-signal")
    );
}

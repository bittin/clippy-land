use super::*;

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

    dispatch(&mut app, Message::ClipboardChanged(repeated.clone()));

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

    dispatch(&mut app, Message::ClipboardChanged(image_entry(42)));
    assert_eq!(app.thumbnail_handles.len(), 1);

    dispatch(&mut app, Message::RemoveHistory(0));
    assert!(app.history.is_empty());
    assert!(app.thumbnail_handles.is_empty());
}

#[test]
fn clipboard_changed_recomputes_filtered_indices_cache() {
    let mut app = AppModel::default();
    app.search_query = "ap".into();
    app.recompute_filtered_indices();
    assert!(app.filtered_indices.is_empty());

    dispatch(
        &mut app,
        Message::ClipboardChanged(ClipboardEntry::Text("apple".into())),
    );

    assert_eq!(app.filtered_indices, vec![0]);
}

#[test]
fn clear_history_clears_thumbnail_handles() {
    let mut app = AppModel::default();

    dispatch(&mut app, Message::ClipboardChanged(image_entry(7)));
    assert_eq!(app.thumbnail_handles.len(), 1);

    dispatch(&mut app, Message::ClearHistory);
    assert!(app.history.is_empty());
    assert!(app.thumbnail_handles.is_empty());
}

#[test]
fn clear_history_retains_pinned_image_thumbnail_handles() {
    let mut app = AppModel::default();

    dispatch(&mut app, Message::ClipboardChanged(image_entry(7)));
    dispatch(&mut app, Message::TogglePin(0));
    assert_eq!(app.history.len(), 1);
    assert!(app.history[0].pinned);
    assert_eq!(app.thumbnail_handles.len(), 1);

    dispatch(&mut app, Message::ClearHistory);

    assert_eq!(app.history.len(), 1);
    assert!(app.history[0].pinned);
    assert_eq!(app.thumbnail_handles.len(), 1);
}

#[test]
fn open_text_overlay_sets_overlay_index_for_text_entry() {
    let mut app = AppModel::default();
    app.history
        .push_back(text_item("first line\nsecond line", false));

    dispatch(&mut app, Message::OpenTextOverlay(0));

    assert_eq!(app.text_overlay_index, Some(0));
}

#[test]
fn open_text_overlay_ignores_image_entries() {
    let mut app = AppModel::default();
    app.history.push_back(HistoryItem {
        entry: image_entry(9),
        pinned: false,
    });

    dispatch(&mut app, Message::OpenTextOverlay(0));

    assert!(app.text_overlay_index.is_none());
}

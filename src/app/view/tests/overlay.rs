use super::*;

#[test]
fn selected_text_overlay_returns_full_text_for_open_multiline_entry() {
    let mut app = AppModel::default();
    push_text(&mut app, "first line\nsecond line");
    app.recompute_filtered_indices();
    app.text_overlay_index = Some(0);

    let overlay = selected_text_overlay(&app, &[0]);

    assert_eq!(overlay.as_deref(), Some("first line\nsecond line"));
}

#[test]
fn selected_text_overlay_returns_none_for_short_single_line_entry() {
    let mut app = AppModel::default();
    push_text(&mut app, "short line");
    app.recompute_filtered_indices();
    app.text_overlay_index = Some(0);

    let overlay = selected_text_overlay(&app, &[0]);

    assert!(overlay.is_none());
}

#[test]
fn selected_text_overlay_returns_none_without_open_index() {
    let mut app = AppModel::default();
    push_text(
        &mut app,
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnop",
    );
    app.recompute_filtered_indices();

    let overlay = selected_text_overlay(&app, &[0]);

    assert!(overlay.is_none());
}

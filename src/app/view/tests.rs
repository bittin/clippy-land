use super::summary::{summarize_one_line, summarize_one_line_with_limit};
use crate::app::AppModel;
use crate::app::model::HistoryItem;
use crate::app::view::popup::filtered_indices;
use crate::services::clipboard::ClipboardEntry;

#[test]
fn summarizes_first_nonempty_line() {
    let input = "\n   \n  hello world  \nsecond line";
    assert_eq!(summarize_one_line(input), "hello world");
}

#[test]
fn truncates_long_lines_with_ellipsis() {
    let input = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnop";
    assert_eq!(
        summarize_one_line(input),
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefg…"
    );
}

#[test]
fn summarize_with_custom_limit_allows_longer_expansion() {
    let input = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnop";
    assert_eq!(
        summarize_one_line_with_limit(input, 150),
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnop"
    );
}

#[test]
fn summarize_with_custom_limit_truncates_to_requested_length() {
    let input = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnop";
    assert_eq!(
        summarize_one_line_with_limit(input, 20),
        "abcdefghijklmnopqrs…"
    );
}

#[test]
fn returns_empty_for_blank_text() {
    assert_eq!(summarize_one_line("\n  \n\t"), "");
}

// ── filtered_indices ─────────────────────────────────────────────────────────

fn push_text(app: &mut AppModel, text: &str) {
    app.history.push_back(HistoryItem {
        entry: ClipboardEntry::Text(text.to_string()),
        pinned: false,
    });
}

fn push_image(app: &mut AppModel, mime: &str) {
    app.history.push_back(HistoryItem {
        entry: ClipboardEntry::Image {
            mime: mime.to_string(),
            bytes: vec![],
            hash: 0,
            thumbnail_png: None,
        },
        pinned: false,
    });
}

#[test]
fn empty_query_returns_all_indices() {
    let mut app = AppModel::default();
    push_text(&mut app, "alpha");
    push_text(&mut app, "beta");
    push_text(&mut app, "gamma");

    let indices = filtered_indices(&app);

    assert_eq!(indices, vec![0, 1, 2]);
}

#[test]
fn query_filters_text_case_insensitively() {
    let mut app = AppModel::default();
    push_text(&mut app, "Hello World"); // idx 0 – matches "hello"
    push_text(&mut app, "HELLO again"); // idx 1 – matches "hello"
    push_text(&mut app, "goodbye"); // idx 2 – no match
    app.search_query = "hello".into();

    let indices = filtered_indices(&app);

    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn query_filters_image_by_mime() {
    let mut app = AppModel::default();
    push_image(&mut app, "image/png"); // idx 0 – matches "png"
    push_image(&mut app, "image/jpeg"); // idx 1 – no match
    push_text(&mut app, "png file"); // idx 2 – text also matches "png"
    app.search_query = "png".into();

    let indices = filtered_indices(&app);

    assert_eq!(indices, vec![0, 2]);
}

#[test]
fn query_with_no_matches_returns_empty() {
    let mut app = AppModel::default();
    push_text(&mut app, "apple");
    push_text(&mut app, "banana");
    app.search_query = "zzz".into();

    let indices = filtered_indices(&app);

    assert!(indices.is_empty());
}

#[test]
fn filtered_indices_empty_history_returns_empty() {
    let app = AppModel::default();
    // No search query either
    let indices = filtered_indices(&app);
    assert!(indices.is_empty());
}

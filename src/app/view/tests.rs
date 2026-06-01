use super::summary::{summarize_one_line, summarize_one_line_with_limit};
use crate::app::AppModel;
use crate::app::model::HistoryItem;
use crate::app::view::popup::{filtered_indices, selected_text_overlay};
use crate::app::view::row::{RowContent, RowRenderState};
use crate::services::clipboard::ClipboardEntry;
use cosmic::iced::widget::image::Handle as ImageHandle;

#[test]
fn summarizes_first_nonempty_line() {
    let input = "\n   \n  hello world  \nsecond line";
    assert_eq!(summarize_one_line(input), "hello world");
}

#[test]
fn truncates_long_lines_with_ellipsis() {
    let input = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabc";
    assert_eq!(
        summarize_one_line(input),
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyza…"
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

#[test]
fn row_render_state_text_snapshot_keeps_only_needed_summaries() {
    let mut app = AppModel::default();
    let long_text = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
    push_text(&mut app, long_text);

    let state = RowRenderState::from_app(&app, 0, &app.history[0]);

    match state.content {
        RowContent::Text {
            collapsed_summary,
            expanded_summary,
            overlay_available,
        } => {
            assert_eq!(collapsed_summary, summarize_one_line(long_text));
            assert_eq!(
                expanded_summary,
                summarize_one_line_with_limit(long_text, 300)
            );
            assert!(overlay_available);
        }
        RowContent::Image { .. } => panic!("expected text row snapshot"),
    }
}

#[test]
fn row_render_state_image_snapshot_keeps_lightweight_metadata_and_handle() {
    let mut app = AppModel::default();
    let thumbnail = vec![1, 2, 3, 4];
    app.history.push_back(HistoryItem {
        entry: ClipboardEntry::Image {
            mime: "image/png".into(),
            bytes: vec![7; 4096],
            hash: 42,
            thumbnail_png: Some(thumbnail.clone()),
        },
        pinned: true,
    });
    app.thumbnail_handles
        .insert((42, 4096), ImageHandle::from_bytes(thumbnail));

    let state = RowRenderState::from_app(&app, 0, &app.history[0]);

    match state.content {
        RowContent::Image {
            mime,
            bytes_len,
            content_hash,
            thumbnail_handle,
        } => {
            assert_eq!(mime, "image/png");
            assert_eq!(bytes_len, 4096);
            assert_eq!(content_hash, 42);
            assert!(thumbnail_handle.is_some());
            assert!(state.pinned);
        }
        RowContent::Text { .. } => panic!("expected image row snapshot"),
    }
}

#[test]
fn row_render_state_image_snapshot_without_cached_handle_keeps_none() {
    let mut app = AppModel::default();
    app.history.push_back(HistoryItem {
        entry: ClipboardEntry::Image {
            mime: "image/png".into(),
            bytes: vec![7; 4096],
            hash: 777,
            thumbnail_png: Some(vec![1, 2, 3, 4]),
        },
        pinned: false,
    });

    let state = RowRenderState::from_app(&app, 0, &app.history[0]);

    match state.content {
        RowContent::Image {
            thumbnail_handle, ..
        } => {
            assert!(thumbnail_handle.is_none());
        }
        RowContent::Text { .. } => panic!("expected image row snapshot"),
    }
}

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

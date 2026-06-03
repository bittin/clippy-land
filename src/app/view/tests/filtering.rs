use super::*;

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
    push_text(&mut app, "Hello World");
    push_text(&mut app, "HELLO again");
    push_text(&mut app, "goodbye");
    app.search_query = "hello".into();

    let indices = filtered_indices(&app);

    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn query_filters_image_by_mime() {
    let mut app = AppModel::default();
    push_image(&mut app, "image/png");
    push_image(&mut app, "image/jpeg");
    push_text(&mut app, "png file");
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

    let indices = filtered_indices(&app);
    assert!(indices.is_empty());
}

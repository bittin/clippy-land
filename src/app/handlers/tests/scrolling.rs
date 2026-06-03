use super::*;

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

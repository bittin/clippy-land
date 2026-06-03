use super::*;

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

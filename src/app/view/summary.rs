pub(crate) const DEFAULT_MAX_CHARS: usize = 80;
pub(crate) const EXPANDED_MAX_CHARS: usize = 300;

pub(super) fn summarize_one_line(text: &str) -> String {
    summarize_one_line_with_limit(text, DEFAULT_MAX_CHARS)
}

pub(crate) fn text_overlay_available(text: &str) -> bool {
    let collapsed = summarize_one_line(text);
    let expanded = summarize_one_line_with_limit(text, EXPANDED_MAX_CHARS);
    let has_more_nonempty_lines = text
        .lines()
        .skip_while(|line| line.trim().is_empty())
        .skip(1)
        .any(|line| !line.trim().is_empty());

    collapsed != expanded || has_more_nonempty_lines
}

pub(super) fn summarize_one_line_with_limit(text: &str, max_chars: usize) -> String {
    let max_chars = max_chars.max(1);
    let mut line = text
        .lines()
        .map(|line| line.trim_start())
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .trim_end()
        .to_string();
    if line.chars().count() > max_chars {
        line = line.chars().take(max_chars - 1).collect::<String>();
        line.push('…');
    }
    line
}

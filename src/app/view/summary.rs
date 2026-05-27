const DEFAULT_MAX_CHARS: usize = 60;

pub(super) fn summarize_one_line(text: &str) -> String {
    summarize_one_line_with_limit(text, DEFAULT_MAX_CHARS)
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

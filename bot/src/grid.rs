use regex::Regex;
use std::sync::OnceLock;

fn emoji_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<a?:(\w+):\d+>").unwrap())
}

pub fn extract_grid(content: &str) -> Vec<Vec<bool>> {
    let rows: Vec<Vec<bool>> = content
        .split('\n')
        .map(|line| {
            emoji_regex()
                .captures_iter(line)
                .map(|caps| !&caps[1].to_lowercase().contains("air"))
                .collect()
        })
        .collect();

    let width = rows.iter().map(Vec::len).max().unwrap_or(0);

    rows.into_iter()
        .map(|mut row| {
            row.resize(width, false);
            row
        })
        .collect()
}

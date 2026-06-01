use regex::Regex;
use std::sync::OnceLock;

/// A single parsed cell of a message grid: either empty "air" (fluid) or a custom
/// emoji obstacle, remembering the source emoji so the render can reuse its image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Air,
    Emoji { id: u64, animated: bool },
}

impl Cell {
    pub fn is_solid(&self) -> bool {
        matches!(self, Cell::Emoji { .. })
    }
}

fn emoji_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<(a?):(\w+):(\d+)>").unwrap())
}

pub fn extract_grid(content: &str) -> Vec<Vec<Cell>> {
    let rows: Vec<Vec<Cell>> = content
        .split('\n')
        .map(|line| {
            emoji_regex()
                .captures_iter(line)
                .map(|caps| {
                    if caps[2].to_lowercase().contains("air") {
                        Cell::Air
                    } else {
                        Cell::Emoji {
                            id: caps[3].parse().unwrap_or(0),
                            animated: !caps[1].is_empty(),
                        }
                    }
                })
                .collect()
        })
        .collect();

    let width = rows.iter().map(Vec::len).max().unwrap_or(0);

    rows.into_iter()
        .map(|mut row| {
            row.resize(width, Cell::Air);
            row
        })
        .collect()
}

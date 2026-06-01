use crate::grid::Cell;
use engine::VelocityField;
use regex::Regex;
use std::collections::HashMap;

// Components V2 text display raises the per-message text limit from 2000 to 4000.
const CONTENT_LIMIT: usize = 4000;

pub trait ArrowSet {
    fn solid(&self) -> &str;
    fn still(&self) -> &str;
    fn arrow(&self, angle_deg: f32) -> &str;
    fn cell_len(&self) -> usize;
}

/// Custom Discord arrow emojis from `emojis.json` (`arrow_0`..`arrow_359`, keyed by
/// integer degree CCW from the right-pointing direction).
pub struct CustomArrows {
    arrows: Vec<String>,
    solid: String,
    still: String,
    cell_len: usize,
}

impl CustomArrows {
    pub fn load() -> Self {
        let raw = include_str!("../emojis.json");
        let re = Regex::new(r#""arrow_(\d+)"\s*:\s*"(a?\d+)""#).unwrap();

        let mut arrows = vec![String::new(); 360];
        for caps in re.captures_iter(raw) {
            let deg: usize = caps[1].parse().unwrap();
            if deg < 360 {
                // A leading `a` on the id marks an animated emoji: strip it and use the
                // animated markup form `<a:name:id>`.
                arrows[deg] = match caps[2].strip_prefix('a') {
                    Some(id) => format!("<a:arrow_{deg}:{id}>"),
                    None => format!("<:arrow_{deg}:{}>", &caps[2]),
                };
            }
        }

        let cell_len = arrows.iter().map(String::len).max().unwrap_or(0);
        Self {
            arrows,
            solid: "⬛".to_string(),
            still: "⬜".to_string(),
            cell_len,
        }
    }
}

impl ArrowSet for CustomArrows {
    fn solid(&self) -> &str {
        &self.solid
    }

    fn still(&self) -> &str {
        &self.still
    }

    fn arrow(&self, angle_deg: f32) -> &str {
        let deg = angle_deg.rem_euclid(360.0).round() as usize % 360;
        &self.arrows[deg]
    }

    fn cell_len(&self) -> usize {
        self.cell_len
    }
}

/// Look up the source emoji markup for a solid domain voxel, inverting the placement
/// in `sim::build_domain` (`grid.set(margin_x + c, (dh-1) - (margin_y + r), true)`).
fn solid_markup<'a>(
    cells: &[Vec<Cell>],
    margin_x: usize,
    margin_y: usize,
    domain_height: usize,
    solids: &'a HashMap<u64, String>,
    x: usize,
    y: usize,
) -> Option<&'a str> {
    let c = x.checked_sub(margin_x)?;
    let r = (domain_height - 1).checked_sub(margin_y + y)?;
    if let Cell::Emoji { id, .. } = *cells.get(r)?.get(c)? {
        return solids.get(&id).map(String::as_str);
    }
    None
}

pub fn render_field(
    field: &VelocityField,
    arrows: &impl ArrowSet,
    still_threshold: f32,
    cells: &[Vec<Cell>],
    margin_x: usize,
    margin_y: usize,
    solids: &HashMap<u64, String>,
) -> String {
    let mut out = String::new();
    for y in (0..field.height).rev() {
        for x in 0..field.width {
            if field.is_solid(x, y) {
                match solid_markup(cells, margin_x, margin_y, field.height, solids, x, y) {
                    Some(markup) => out.push_str(markup),
                    None => out.push_str(arrows.solid()),
                }
            } else {
                let (ux, uy) = field.at(x, y);
                if field.magnitude(x, y) < still_threshold {
                    out.push_str(arrows.still());
                } else {
                    out.push_str(arrows.arrow(uy.atan2(ux).to_degrees()));
                }
            }
        }
        if y != 0 {
            out.push('\n');
        }
    }
    out
}

/// Upper bound on a re-uploaded obstacle emoji's markup length: `<a:` + name
/// (`e` + up to a 20-digit source id) + `:` + up to a 20-digit emoji id + `>`.
const SOLID_MARKUP_MAX: usize = 48;

pub fn fits(width: usize, height: usize, arrows: &impl ArrowSet) -> bool {
    let cell_len = arrows.cell_len().max(SOLID_MARKUP_MAX);
    height * (width * cell_len + 1) <= CONTENT_LIMIT
}

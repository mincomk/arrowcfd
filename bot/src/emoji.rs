use engine::VelocityField;
use regex::Regex;

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

pub fn render_field(field: &VelocityField, arrows: &impl ArrowSet, still_threshold: f32) -> String {
    let mut out = String::new();
    for y in (0..field.height).rev() {
        for x in 0..field.width {
            if field.is_solid(x, y) {
                out.push_str(arrows.solid());
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

pub fn fits(width: usize, height: usize, arrows: &impl ArrowSet) -> bool {
    height * (width * arrows.cell_len() + 1) <= CONTENT_LIMIT
}

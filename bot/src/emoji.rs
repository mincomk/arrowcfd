use engine::VelocityField;

const CONTENT_LIMIT: usize = 2000;

pub trait ArrowSet {
    fn solid(&self) -> &str;
    fn still(&self) -> &str;
    fn arrow(&self, angle_deg: f32) -> &str;
    fn cell_len(&self) -> usize;
}

pub struct UnicodeArrows;

const UNICODE_DIRECTIONS: [&str; 4] = ["➡\u{fe0f}", "⬆\u{fe0f}", "⬅\u{fe0f}", "⬇\u{fe0f}"];

impl ArrowSet for UnicodeArrows {
    fn solid(&self) -> &str {
        "⬛"
    }

    fn still(&self) -> &str {
        "⬜"
    }

    fn arrow(&self, angle_deg: f32) -> &str {
        let normalized = angle_deg.rem_euclid(360.0);
        let bucket = (normalized / 90.0).round() as usize % UNICODE_DIRECTIONS.len();
        UNICODE_DIRECTIONS[bucket]
    }

    fn cell_len(&self) -> usize {
        2
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

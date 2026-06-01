use engine::Grid;

pub fn build_domain(obstacle: &[Vec<bool>], margin_x: usize, margin_y: usize) -> Grid {
    let oh = obstacle.len();
    let ow = obstacle.first().map_or(0, Vec::len);
    let dw = ow + 2 * margin_x;
    let dh = oh + 2 * margin_y;

    let mut grid = Grid::new(dw, dh);
    for (r, row) in obstacle.iter().enumerate() {
        for (c, &solid) in row.iter().enumerate() {
            if solid {
                grid.set(margin_x + c, (dh - 1) - (margin_y + r), true);
            }
        }
    }
    grid
}

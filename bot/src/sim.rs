use engine::{Grid, VelocityField};

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

/// Refine a domain by the grid complexity multiplier `m`: each coarse voxel becomes a
/// homogeneous `m × m` block, so the solver runs at `m²` the resolution. `m <= 1` is a
/// no-op (the current default behavior).
pub fn refine(grid: &Grid, m: usize) -> Grid {
    if m <= 1 {
        return grid.clone();
    }
    Grid::from_fn(grid.width * m, grid.height * m, |x, y| {
        grid.is_solid(x / m, y / m)
    })
}

/// Merge a fine velocity field back down to the coarse cell count by averaging each
/// `m × m` block. Refinement keeps every block uniformly solid or fluid, so a block is
/// solid iff its top-left voxel is. `m <= 1` returns the field unchanged.
pub fn merge_field(fine: &VelocityField, m: usize) -> VelocityField {
    if m <= 1 {
        return fine.clone();
    }
    let cw = fine.width / m;
    let ch = fine.height / m;
    let mut ux = vec![0.0f32; cw * ch];
    let mut uy = vec![0.0f32; cw * ch];
    let mut solid = vec![false; cw * ch];

    for cy in 0..ch {
        for cx in 0..cw {
            let ci = cy * cw + cx;
            if fine.is_solid(cx * m, cy * m) {
                solid[ci] = true;
                continue;
            }
            let (mut sx, mut sy) = (0.0f32, 0.0f32);
            for dy in 0..m {
                for dx in 0..m {
                    let (vx, vy) = fine.at(cx * m + dx, cy * m + dy);
                    sx += vx;
                    sy += vy;
                }
            }
            let n = (m * m) as f32;
            ux[ci] = sx / n;
            uy[ci] = sy / n;
        }
    }

    VelocityField {
        width: cw,
        height: ch,
        ux,
        uy,
        solid,
    }
}

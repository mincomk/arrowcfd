//! The input voxel grid: a boolean occupancy map where `true` marks a solid
//! voxel (fluid cannot pass) and `false` marks an open voxel (fluid flows).
//!
//! Storage is row-major: index `(x, y)` lives at `y * width + x`.

/// A 2D boolean voxel grid describing solid obstacles for the LBM solver.
#[derive(Clone, Debug)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    /// `true` = solid wall, `false` = open fluid. Length is `width * height`.
    pub solid: Vec<bool>,
}

impl Grid {
    /// An all-fluid grid (no obstacles).
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            solid: vec![false; width * height],
        }
    }

    /// Build a grid from a predicate: `f(x, y) == true` marks a solid voxel.
    pub fn from_fn(width: usize, height: usize, f: impl Fn(usize, usize) -> bool) -> Self {
        let mut solid = vec![false; width * height];
        for y in 0..height {
            for x in 0..width {
                solid[y * width + x] = f(x, y);
            }
        }
        Self {
            width,
            height,
            solid,
        }
    }

    #[inline]
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    #[inline]
    pub fn is_solid(&self, x: usize, y: usize) -> bool {
        self.solid[self.index(x, y)]
    }

    /// Mark a voxel solid (`true`) or open (`false`).
    pub fn set(&mut self, x: usize, y: usize, solid: bool) {
        let i = self.index(x, y);
        self.solid[i] = solid;
    }
}

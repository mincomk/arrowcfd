//! The solver output: a macroscopic velocity vector for every voxel.

/// A snapshot of the macroscopic velocity field. `ux`/`uy` are row-major
/// (`y * width + x`). Solid voxels report `(0, 0)`.
#[derive(Clone, Debug)]
pub struct VelocityField {
    pub width: usize,
    pub height: usize,
    pub ux: Vec<f32>,
    pub uy: Vec<f32>,
    /// Solid mask, mirrors the input grid.
    pub solid: Vec<bool>,
}

impl VelocityField {
    #[inline]
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Velocity vector at `(x, y)`.
    #[inline]
    pub fn at(&self, x: usize, y: usize) -> (f32, f32) {
        let i = self.index(x, y);
        (self.ux[i], self.uy[i])
    }

    /// Speed (vector magnitude) at `(x, y)`.
    #[inline]
    pub fn magnitude(&self, x: usize, y: usize) -> f32 {
        let (ux, uy) = self.at(x, y);
        (ux * ux + uy * uy).sqrt()
    }

    #[inline]
    pub fn is_solid(&self, x: usize, y: usize) -> bool {
        self.solid[self.index(x, y)]
    }

    /// Largest speed over all fluid voxels (useful for normalising a colormap).
    pub fn max_magnitude(&self) -> f32 {
        let mut m = 0.0f32;
        for i in 0..self.ux.len() {
            let s = (self.ux[i] * self.ux[i] + self.uy[i] * self.uy[i]).sqrt();
            if s > m {
                m = s;
            }
        }
        m
    }
}

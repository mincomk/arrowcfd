//! A simple voxel-based 2D Lattice-Boltzmann (D2Q9, BGK) incompressible CFD
//! engine built on CubeCL.
//!
//! Give it a boolean [`Grid`] of solid voxels, a few [`FluidParams`], and a
//! flow angle; step the [`Solver`] and read back a [`VelocityField`] — the
//! macroscopic velocity vector for every open voxel — as often as you like.
//!
//! The solver is generic over any CubeCL [`Runtime`](cubecl::prelude::Runtime),
//! so the same code runs on the CPU backend or any GPU backend (wgpu, CUDA,
//! HIP) by choosing a runtime type. See `examples/flow.rs`.

mod field;
mod grid;
pub mod lbm;
mod params;
mod solver;

pub use field::VelocityField;
pub use grid::Grid;
pub use params::FluidParams;
pub use solver::Solver;

//! D2Q9 BGK Lattice-Boltzmann kernels (CubeCL).
//!
//! Scheme: fused "pull" collide-stream. Each fluid node gathers the nine
//! populations streaming into it from its neighbours (applying halfway
//! bounce-back where a neighbour is a wall), computes the macroscopic moments,
//! relaxes towards equilibrium (BGK), and writes to a second buffer. The two
//! buffers are ping-ponged each step.
//!
//! Structure-of-arrays layout (coalesced): `idx(dir, x, y) = dir*(NX*NY) + y*NX + x`.
//!
//! Boundary model — generalised from the lid-driven cavity to uniform
//! free-stream flow at an arbitrary angle past arbitrary solid voxels:
//!   FLUID      : ordinary fluid node, collides and streams.
//!   WALL       : solid voxel, halfway bounce-back.
//!   FREESTREAM : open domain-border node, held at the equilibrium of the
//!                imposed free-stream velocity every step. Acts as a velocity
//!                inlet on upstream sides and a soft outflow downstream.

use cubecl::prelude::*;

/// Flag values stored in the `flags` array.
pub const FLUID: u32 = 0;
pub const WALL: u32 = 1;
pub const FREESTREAM: u32 = 2;

// D2Q9 lattice constants (host copies; uploaded to the device once).
pub const CX: [i32; 9] = [0, 1, 0, -1, 0, 1, -1, -1, 1];
pub const CY: [i32; 9] = [0, 0, 1, 0, -1, 1, 1, -1, -1];
pub const OPP: [u32; 9] = [0, 3, 4, 1, 2, 7, 8, 5, 6];
pub const W: [f32; 9] = [
    4.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
];

/// Discrete equilibrium distribution for population `i`.
///   feq_i = w_i * rho * (1 + 3 (c_i·u) + 4.5 (c_i·u)^2 - 1.5 |u|^2)
#[cube]
pub fn feq(rho: f32, ux: f32, uy: f32, cxi: f32, cyi: f32, wi: f32) -> f32 {
    let eu = cxi * ux + cyi * uy;
    let usq = 1.5 * (ux * ux + uy * uy);
    wi * rho * (1.0 + 3.0 * eu + 4.5 * eu * eu - usq)
}

/// Gather population `i` streaming into node (x, y), with bounce-back at walls.
/// FLUID and FREESTREAM neighbours both stream ordinarily (FREESTREAM nodes
/// always carry a valid equilibrium distribution).
#[cube]
fn pull(
    f: &Array<f32>,
    flags: &Array<u32>,
    cx: &Array<i32>,
    cy: &Array<i32>,
    opp: &Array<u32>,
    x: u32,
    y: u32,
    i: usize,
    #[comptime] nx: u32,
    #[comptime] ny: u32,
) -> f32 {
    let plane = (nx * ny) as usize;
    let here = (y * nx + x) as usize;

    // Source node we pull population i from: (x - c_i). Fluid nodes are strictly
    // interior (the domain border guards the edges) so this never goes out of
    // range.
    let sx = (x as i32 - cx[i]) as u32;
    let sy = (y as i32 - cy[i]) as u32;
    let src = (sy * nx + sx) as usize;

    let mut fin = f32::new(0.0);
    if flags[src] == WALL {
        // Halfway bounce-back: the population we sent toward the wall (opp[i])
        // comes back as population i.
        fin = f[opp[i] as usize * plane + here];
    } else {
        // Ordinary streaming from a FLUID or FREESTREAM neighbour.
        fin = f[i * plane + src];
    }
    fin
}

/// One LBM timestep for one node: collide + stream (pull form).
#[cube(launch)]
pub fn collide_stream(
    f: &Array<f32>,
    f_new: &mut Array<f32>,
    flags: &Array<u32>,
    cx: &Array<i32>,
    cy: &Array<i32>,
    opp: &Array<u32>,
    w: &Array<f32>,
    omega: f32,
    ux0: f32,
    uy0: f32,
    #[comptime] nx: u32,
    #[comptime] ny: u32,
) {
    let x = ABSOLUTE_POS_X;
    let y = ABSOLUTE_POS_Y;

    if x < nx && y < ny {
        let plane = (nx * ny) as usize;
        let here = (y * nx + x) as usize;
        let flag = flags[here];

        if flag == FREESTREAM {
            // Impose the free-stream: reset populations to equilibrium at the
            // target velocity (rho = 1). This holds every step in both buffers.
            for i in 0..9usize {
                f_new[i * plane + here] = feq(1.0, ux0, uy0, cx[i] as f32, cy[i] as f32, w[i]);
            }
        } else if flag == FLUID {
            // --- Pass 1: macroscopic moments ---------------------------------
            let mut rho = f32::new(0.0);
            let mut mx = f32::new(0.0);
            let mut my = f32::new(0.0);
            for i in 0..9usize {
                let fin = pull(f, flags, cx, cy, opp, x, y, i, nx, ny);
                rho += fin;
                mx += fin * (cx[i] as f32);
                my += fin * (cy[i] as f32);
            }
            let ux = mx / rho;
            let uy = my / rho;

            // --- Pass 2: BGK collision + write -------------------------------
            for i in 0..9usize {
                let fin = pull(f, flags, cx, cy, opp, x, y, i, nx, ny);
                let feqi = feq(rho, ux, uy, cx[i] as f32, cy[i] as f32, w[i]);
                f_new[i * plane + here] = fin - omega * (fin - feqi);
            }
        }
        // WALL nodes are left unwritten: their populations are never read as a
        // stream source (bounce-back reads the fluid node's own populations).
    }
}

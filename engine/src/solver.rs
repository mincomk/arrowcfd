//! The LBM solver: owns the device buffers, advances the simulation, and
//! reads back the macroscopic velocity field on demand.

use cubecl::prelude::*;
use cubecl::server::Handle;

use crate::field::VelocityField;
use crate::grid::Grid;
use crate::lbm::{self, FLUID, FREESTREAM, WALL};
use crate::params::FluidParams;

/// A generic D2Q9 BGK Lattice-Boltzmann solver over any CubeCL `Runtime`.
pub struct Solver<R: Runtime> {
    client: ComputeClient<R>,
    // Ping-pong population buffers (9 * nx * ny floats each).
    f_a: Handle,
    f_b: Handle,
    flags: Handle,
    cx: Handle,
    cy: Handle,
    opp: Handle,
    w: Handle,
    nx: u32,
    ny: u32,
    flen: usize,
    omega: f32,
    ux0: f32,
    uy0: f32,
    solid: Vec<bool>,
    cube_count: CubeCount,
    cube_dim: CubeDim,
}

impl<R: Runtime> Solver<R> {
    /// Build a solver for `grid` with the given fluid parameters on `device`.
    pub fn new(device: &R::Device, grid: &Grid, params: &FluidParams) -> Self {
        let nx = grid.width as u32;
        let ny = grid.height as u32;
        let plane = (nx * ny) as usize;
        let flen = 9 * plane;

        let (ux0, uy0) = params.velocity();

        // Relaxation time from the target Reynolds number.
        //   nu = |U| * L / Re,  tau = 3 nu + 1/2,  omega = 1/tau.
        let l = nx.min(ny) as f32;
        let nu = params.inflow_speed * l / params.reynolds;
        let tau = 3.0 * nu + 0.5;
        let omega = 1.0 / tau;
        println!("LBM: nu = {nu:.5}, tau = {tau:.5}, omega = {omega:.5}");
        assert!(tau > 0.5, "tau must exceed 0.5 for stability");

        // --- Flags: user walls + an open border held at the free-stream ------
        let mut flags = vec![FLUID; plane];
        for y in 0..grid.height {
            for x in 0..grid.width {
                let n = y * grid.width + x;
                let edge = x == 0 || x == grid.width - 1 || y == 0 || y == grid.height - 1;
                if grid.solid[n] {
                    flags[n] = WALL;
                } else if edge {
                    flags[n] = FREESTREAM;
                }
            }
        }

        // --- Initialise both buffers -----------------------------------------
        // WALL nodes: rest weights (never read anyway). FLUID & FREESTREAM:
        // equilibrium at the free-stream velocity to seed the flow.
        let mut f0 = vec![0.0f32; flen];
        for n in 0..plane {
            let wall = flags[n] == WALL;
            for i in 0..9usize {
                f0[i * plane + n] = if wall {
                    lbm::W[i]
                } else {
                    feq_host(1.0, ux0, uy0, lbm::CX[i] as f32, lbm::CY[i] as f32, lbm::W[i])
                };
            }
        }

        let client = R::client(device);
        let f_a = client.create_from_slice(f32::as_bytes(&f0));
        let f_b = client.create_from_slice(f32::as_bytes(&f0));
        let flags_gpu = client.create_from_slice(u32::as_bytes(&flags));
        let cx = client.create_from_slice(i32::as_bytes(&lbm::CX));
        let cy = client.create_from_slice(i32::as_bytes(&lbm::CY));
        let opp = client.create_from_slice(u32::as_bytes(&lbm::OPP));
        let w = client.create_from_slice(f32::as_bytes(&lbm::W));

        let cube_dim = CubeDim::new_3d(16, 16, 1);
        let cube_count = CubeCount::Static(nx.div_ceil(16), ny.div_ceil(16), 1);

        Self {
            client,
            f_a,
            f_b,
            flags: flags_gpu,
            cx,
            cy,
            opp,
            w,
            nx,
            ny,
            flen,
            omega,
            ux0,
            uy0,
            solid: grid.solid.clone(),
            cube_count,
            cube_dim,
        }
    }

    /// Advance the simulation by a single timestep.
    pub fn step(&mut self) {
        let plane = (self.nx * self.ny) as usize;
        unsafe {
            lbm::collide_stream::launch::<R>(
                &self.client,
                self.cube_count.clone(),
                self.cube_dim,
                ArrayArg::from_raw_parts(self.f_a.clone(), self.flen),
                ArrayArg::from_raw_parts(self.f_b.clone(), self.flen),
                ArrayArg::from_raw_parts(self.flags.clone(), plane),
                ArrayArg::from_raw_parts(self.cx.clone(), 9),
                ArrayArg::from_raw_parts(self.cy.clone(), 9),
                ArrayArg::from_raw_parts(self.opp.clone(), 9),
                ArrayArg::from_raw_parts(self.w.clone(), 9),
                self.omega,
                self.ux0,
                self.uy0,
                self.nx,
                self.ny,
            );
        }
        std::mem::swap(&mut self.f_a, &mut self.f_b);
    }

    /// Advance the simulation by `n` timesteps.
    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    /// Read back the current distributions and recompute the macroscopic
    /// velocity vector for every voxel. Solid voxels report `(0, 0)`.
    pub fn velocity_field(&self) -> VelocityField {
        let plane = (self.nx * self.ny) as usize;
        let bytes = self.client.read_one(self.f_a.clone()).unwrap();
        let f = f32::from_bytes(&bytes);

        let mut ux = vec![0.0f32; plane];
        let mut uy = vec![0.0f32; plane];
        for n in 0..plane {
            if self.solid[n] {
                continue;
            }
            let mut rho = 0.0f32;
            let mut mx = 0.0f32;
            let mut my = 0.0f32;
            for i in 0..9usize {
                let fi = f[i * plane + n];
                rho += fi;
                mx += fi * lbm::CX[i] as f32;
                my += fi * lbm::CY[i] as f32;
            }
            if rho > 0.0 {
                ux[n] = mx / rho;
                uy[n] = my / rho;
            }
        }

        VelocityField {
            width: self.nx as usize,
            height: self.ny as usize,
            ux,
            uy,
            solid: self.solid.clone(),
        }
    }
}

/// Host-side copy of the equilibrium distribution (mirrors `lbm::feq`).
fn feq_host(rho: f32, ux: f32, uy: f32, cxi: f32, cyi: f32, wi: f32) -> f32 {
    let eu = cxi * ux + cyi * uy;
    let usq = 1.5 * (ux * ux + uy * uy);
    wi * rho * (1.0 + 3.0 * eu + 4.5 * eu * eu - usq)
}

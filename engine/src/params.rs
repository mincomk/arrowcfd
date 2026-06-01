//! Fluid and flow parameters for the LBM solver. All quantities are in
//! lattice units (the LBM convention): one lattice spacing and one timestep
//! are both unity.

/// Physical parameters controlling the simulated flow.
#[derive(Clone, Copy, Debug)]
pub struct FluidParams {
    /// Reynolds number. Together with the inflow speed and a characteristic
    /// length it sets the kinematic viscosity (and hence the relaxation time).
    pub reynolds: f32,
    /// Free-stream / inflow speed magnitude `|U|` in lattice units. Keep the
    /// Mach number `|U| / c_s` small (`c_s = 1/sqrt(3) ≈ 0.577`) for stability;
    /// values around `0.05..0.1` are typical.
    pub inflow_speed: f32,
    /// Flow direction in degrees, measured counter-clockwise from `+x`.
    /// `0.0` is the default left → right flow.
    pub flow_angle_deg: f32,
}

impl Default for FluidParams {
    fn default() -> Self {
        Self {
            reynolds: 100.0,
            inflow_speed: 0.1,
            flow_angle_deg: 0.0,
        }
    }
}

impl FluidParams {
    /// Free-stream velocity components `(ux, uy)` derived from speed and angle.
    pub fn velocity(&self) -> (f32, f32) {
        let theta = self.flow_angle_deg.to_radians();
        (self.inflow_speed * theta.cos(), self.inflow_speed * theta.sin())
    }
}

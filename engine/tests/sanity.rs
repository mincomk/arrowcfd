//! Headless numerical sanity checks for the LBM engine on the CPU runtime.

use engine::{FluidParams, Grid, Solver};

type Rt = cubecl::cpu::CpuRuntime;

/// An empty domain should converge to uniform free-stream flow everywhere.
#[test]
fn empty_domain_is_uniform_freestream() {
    let grid = Grid::new(48, 32);
    let params = FluidParams {
        reynolds: 100.0,
        inflow_speed: 0.1,
        flow_angle_deg: 0.0,
    };
    let device = Default::default();
    let mut solver = Solver::<Rt>::new(&device, &grid, &params);
    solver.step_n(400);

    let field = solver.velocity_field();
    // Sample a few interior voxels; they should match (inflow_speed, 0).
    for &(x, y) in &[(24, 16), (10, 8), (40, 24)] {
        let (ux, uy) = field.at(x, y);
        assert!(ux.is_finite() && uy.is_finite(), "non-finite velocity");
        assert!(
            (ux - 0.1).abs() < 0.01,
            "ux at ({x},{y}) = {ux}, expected ~0.1"
        );
        assert!(uy.abs() < 0.01, "uy at ({x},{y}) = {uy}, expected ~0");
    }
}

/// An obstacle must perturb the flow: fluid is slowed somewhere relative to the
/// free-stream, and solid voxels report zero velocity.
#[test]
fn obstacle_perturbs_flow() {
    let (w, h) = (64usize, 40usize);
    let (cx, cy, r) = (18.0f32, 20.0f32, 6.0f32);
    let grid = Grid::from_fn(w, h, |x, y| {
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        dx * dx + dy * dy <= r * r
    });
    let params = FluidParams {
        reynolds: 100.0,
        inflow_speed: 0.1,
        flow_angle_deg: 0.0,
    };
    let device = Default::default();
    let mut solver = Solver::<Rt>::new(&device, &grid, &params);
    solver.step_n(500);

    let field = solver.velocity_field();

    // Solid voxel reports zero.
    assert_eq!(field.at(18, 20), (0.0, 0.0));

    // Just behind the obstacle the flow should be markedly slower than inflow.
    let wake = field.magnitude(24, 20);
    assert!(wake.is_finite());
    assert!(
        wake < 0.08,
        "wake speed {wake} not slowed below free-stream 0.1"
    );

    // All velocities finite (stability check).
    assert!(field.ux.iter().all(|v| v.is_finite()));
    assert!(field.uy.iter().all(|v| v.is_finite()));
}

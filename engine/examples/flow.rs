//! Realtime visualisation of the LBM engine: uniform flow past a circular
//! obstacle (the classic Kármán-vortex-street setup), shown live in a window.
//!
//! Run with:
//!     cargo run --release --example flow
//!
//! The window shows a velocity-magnitude heatmap (blue = slow, red = fast)
//! with a downsampled quiver of velocity vectors (arrows) overlaid in black.
//! Solid voxels are drawn in dark grey. Press Escape or close the window to
//! quit. Edit `FLOW_ANGLE_DEG` below to send the stream in at an angle.

use engine::{FluidParams, Grid, Solver, VelocityField};
use minifb::{Key, Window, WindowOptions};
use plotters::prelude::*;

// CubeCL CPU runtime — swap for `cubecl::wgpu::WgpuRuntime` (+ the "wgpu"
// feature) to run the identical engine on a GPU.
type Rt = cubecl::cpu::CpuRuntime;

// ---- Domain & display -------------------------------------------------------
const GW: usize = 220; // grid width  (voxels)
const GH: usize = 120; // grid height (voxels)
const SCALE: usize = 5; // pixels per voxel
const WW: usize = GW * SCALE;
const WH: usize = GH * SCALE;

// ---- Flow -------------------------------------------------------------------
const REYNOLDS: f32 = 120.0;
const INFLOW_SPEED: f32 = 0.1;
const FLOW_ANGLE_DEG: f32 = 0.0; // 0 = left -> right; try 30.0 for a tilted stream

// ---- Time stepping ----------------------------------------------------------
const STEPS_PER_FRAME: usize = 10;

// ---- Quiver -----------------------------------------------------------------
const ARROW_EVERY: usize = 10; // sample one arrow per this many voxels
const ARROW_SCALE: f32 = 220.0; // pixels of arrow per unit lattice speed

fn main() {
    // A circular obstacle placed in the left third of the domain.
    let cx = GW as f32 * 0.30;
    let cy = GH as f32 * 0.5;
    let radius = GH as f32 * 0.16;
    let grid = Grid::from_fn(GW, GH, |x, y| {
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        dx * dx + dy * dy <= radius * radius
    });

    let params = FluidParams {
        reynolds: REYNOLDS,
        inflow_speed: INFLOW_SPEED,
        flow_angle_deg: FLOW_ANGLE_DEG,
    };

    let device = Default::default();
    let mut solver = Solver::<Rt>::new(&device, &grid, &params);

    let mut window = Window::new(
        "arrowcfd — LBM flow (Esc to quit)",
        WW,
        WH,
        WindowOptions::default(),
    )
    .expect("failed to create window");
    window.set_target_fps(60);

    // Colormap reference speed: a bit above the free-stream so accelerated
    // flow around the obstacle still has colour headroom.
    let ref_speed = INFLOW_SPEED * 1.8;

    let mut rgb = vec![0u8; WW * WH * 3];
    let mut frame = vec![0u32; WW * WH];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        solver.step_n(STEPS_PER_FRAME);
        let field = solver.velocity_field();

        paint_heatmap(&field, &mut rgb, ref_speed);
        draw_quiver(&field, &mut rgb);
        pack_argb(&rgb, &mut frame);

        window
            .update_with_buffer(&frame, WW, WH)
            .expect("failed to update window");
    }
}

/// Fill the RGB buffer with a velocity-magnitude colormap (one voxel = SCALE^2
/// pixels). Image rows run top-to-bottom while the grid's y runs bottom-to-top,
/// so we flip y.
fn paint_heatmap(field: &VelocityField, rgb: &mut [u8], ref_speed: f32) {
    for py in 0..WH {
        let gy = GH - 1 - (py / SCALE).min(GH - 1);
        for px in 0..WW {
            let gx = (px / SCALE).min(GW - 1);
            let o = (py * WW + px) * 3;
            let (r, g, b) = if field.is_solid(gx, gy) {
                (40u8, 40u8, 40u8)
            } else {
                let t = (field.magnitude(gx, gy) / ref_speed).clamp(0.0, 1.0);
                colormap(t)
            };
            rgb[o] = r;
            rgb[o + 1] = g;
            rgb[o + 2] = b;
        }
    }
}

/// Overlay velocity vectors as black arrows using plotters, drawing directly
/// onto the already-painted RGB buffer.
fn draw_quiver(field: &VelocityField, rgb: &mut [u8]) {
    let root = BitMapBackend::with_buffer(rgb, (WW as u32, WH as u32)).into_drawing_area();

    let half = (SCALE / 2) as i32;
    for gy in (0..GH).step_by(ARROW_EVERY) {
        for gx in (0..GW).step_by(ARROW_EVERY) {
            if field.is_solid(gx, gy) {
                continue;
            }
            let (ux, uy) = field.at(gx, gy);
            // Voxel centre in image pixels (flip y for display).
            let x0 = (gx * SCALE) as i32 + half;
            let y0 = ((GH - 1 - gy) * SCALE) as i32 + half;
            // Arrow tip: +x right, +y up (so subtract in image space).
            let x1 = x0 + (ux * ARROW_SCALE) as i32;
            let y1 = y0 - (uy * ARROW_SCALE) as i32;

            root.draw(&PathElement::new(vec![(x0, y0), (x1, y1)], BLACK))
                .ok();
            // Small dot marks the arrow tip (direction of flow).
            root.draw(&Circle::new((x1, y1), 1, BLACK.filled())).ok();
        }
    }
    root.present().ok();
}

/// Pack an RGB byte buffer into minifb's 0RGB `u32` pixels.
fn pack_argb(rgb: &[u8], frame: &mut [u32]) {
    for (px, chunk) in frame.iter_mut().zip(rgb.chunks_exact(3)) {
        *px = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
    }
}

/// Blue -> cyan -> green -> yellow -> red ramp for t in [0, 1].
fn colormap(t: f32) -> (u8, u8, u8) {
    let r = (1.5 - (4.0 * t - 3.0).abs()).clamp(0.0, 1.0);
    let g = (1.5 - (4.0 * t - 2.0).abs()).clamp(0.0, 1.0);
    let b = (1.5 - (4.0 * t - 1.0).abs()).clamp(0.0, 1.0);
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

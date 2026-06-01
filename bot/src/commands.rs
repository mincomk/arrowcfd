use crate::emoji::{self, CustomArrows};
use crate::{sim, Context, Error};
use engine::{FluidParams, Solver};
use poise::serenity_prelude as serenity;
use std::time::Duration;

const INFLOW_SPEED: f32 = 0.1;
const MAX_UPDATES: usize = 120;
const UPDATE_INTERVAL: Duration = Duration::from_millis(800);

#[derive(poise::Modal)]
#[name = "Flow Simulation"]
struct SimModal {
    #[name = "Margin X"]
    margin_x: Option<String>,
    #[name = "Margin Y"]
    margin_y: Option<String>,
    #[name = "Flow Angle (deg)"]
    flow_angle: Option<String>,
    #[name = "Reynolds"]
    reynolds: Option<String>,
    #[name = "Steps / update"]
    steps: Option<String>,
}

fn parse_or<T: std::str::FromStr>(value: Option<String>, default: T) -> T {
    value
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(default)
}

#[poise::command(context_menu_command = "Solve")]
pub async fn solve(ctx: Context<'_>, msg: serenity::Message) -> Result<(), Error> {
    let grid = crate::grid::extract_grid(&msg.content);
    if grid.is_empty() || grid[0].is_empty() {
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content("No grid found."),
        )
        .await?;
        return Ok(());
    }

    let poise::Context::Application(actx) = ctx else {
        return Ok(());
    };
    let defaults = SimModal {
        margin_x: Some("5".to_string()),
        margin_y: Some("5".to_string()),
        flow_angle: Some("0".to_string()),
        reynolds: Some("100".to_string()),
        steps: Some("20".to_string()),
    };
    let Some(data) =
        poise::execute_modal::<_, _, SimModal>(actx, Some(defaults), Some(Duration::from_secs(300)))
            .await?
    else {
        return Ok(());
    };

    let margin_x = parse_or(data.margin_x, 5usize);
    let margin_y = parse_or(data.margin_y, 5usize);
    let params = FluidParams {
        reynolds: parse_or(data.reynolds, 100.0),
        inflow_speed: INFLOW_SPEED,
        flow_angle_deg: parse_or(data.flow_angle, 0.0),
    };
    let steps = parse_or(data.steps, 20usize).max(1);

    let domain = sim::build_domain(&grid, margin_x, margin_y);
    let arrows = CustomArrows::load();
    if !emoji::fits(domain.width, domain.height, &arrows) {
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content("Domain too large to render; reduce the grid or margins."),
        )
        .await?;
        return Ok(());
    }

    let still_threshold = params.inflow_speed * 0.05;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(1);

    std::thread::spawn(move || {
        let mut solver =
            Solver::<cubecl::cpu::CpuRuntime>::new(&Default::default(), &domain, &params);
        for _ in 0..MAX_UPDATES {
            solver.step_n(steps);
            let content = emoji::render_field(&solver.velocity_field(), &arrows, still_threshold);
            if tx.blocking_send(content).is_err() {
                break;
            }
        }
    });

    let Some(content) = rx.recv().await else {
        return Ok(());
    };
    let handle = ctx
        .send(poise::CreateReply::default().content(content))
        .await?;

    while let Some(content) = rx.recv().await {
        handle
            .edit(ctx, poise::CreateReply::default().content(content))
            .await?;
        tokio::time::sleep(UPDATE_INTERVAL).await;
    }

    Ok(())
}

use crate::emoji::{self, CustomArrows};
use crate::{sim, Context, Error};
use engine::{FluidParams, Solver};
use poise::serenity_prelude as serenity;
use poise::Modal as _;
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

    // Send the modal manually (rather than poise::execute_modal) so we can capture the
    // modal-submit interaction. Editing *its* original response via `PATCH .../@original`
    // is the only edit path that keeps custom emoji rendered (discord-api-docs#4431), and
    // unlike follow-ups it has no per-interaction message cap.
    let defaults = SimModal {
        margin_x: Some("2".to_string()),
        margin_y: Some("2".to_string()),
        flow_angle: Some("0".to_string()),
        reynolds: Some("100".to_string()),
        steps: Some("20".to_string()),
    };
    let custom_id = actx.interaction.id.to_string();
    actx.interaction
        .create_response(
            actx.serenity_context,
            SimModal::create(Some(defaults), custom_id.clone()),
        )
        .await?;

    let Some(modal) = serenity::ModalInteractionCollector::new(&actx.serenity_context.shard)
        .filter(move |mi| mi.data.custom_id == custom_id)
        .timeout(Duration::from_secs(300))
        .await
    else {
        return Ok(());
    };
    let data = SimModal::parse(modal.data.clone())?;

    let margin_x = parse_or(data.margin_x, 2usize);
    let margin_y = parse_or(data.margin_y, 2usize);
    let params = FluidParams {
        reynolds: parse_or(data.reynolds, 100.0),
        inflow_speed: INFLOW_SPEED,
        flow_angle_deg: parse_or(data.flow_angle, 0.0),
    };
    let steps = parse_or(data.steps, 20usize).max(1);

    let domain = sim::build_domain(&grid, margin_x, margin_y);
    let arrows = CustomArrows::load();
    if !emoji::fits(domain.width, domain.height, &arrows) {
        modal
            .create_response(
                actx.serenity_context,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("Domain too large to render; reduce the grid or margins."),
                ),
            )
            .await?;
        return Ok(());
    }

    let http = actx.serenity_context.http.as_ref();
    let app_id = actx.interaction.application_id;

    // Send the initial Components V2 response immediately. This both satisfies the 3s
    // interaction-callback deadline (without racing the solver) and sets IS_COMPONENTS_V2,
    // which is immutable after creation. Real frames stream in via @original edits below.
    send_v2_initial_response(http, modal.id, &modal.token, "Simulating…").await?;

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

    while let Some(content) = rx.recv().await {
        edit_v2_original(http, app_id, &modal.token, &content).await?;
        tokio::time::sleep(UPDATE_INTERVAL).await;
    }

    Ok(())
}

const IS_COMPONENTS_V2: u64 = 1 << 15;
const TEXT_DISPLAY: u8 = 10;

fn text_display_components(content: &str) -> serde_json::Value {
    serde_json::json!([{ "type": TEXT_DISPLAY, "content": content }])
}

/// Initial interaction response (callback type 4) carrying a Components V2 text display.
/// Built manually because serenity 0.12 can't emit the `IS_COMPONENTS_V2` flag.
async fn send_v2_initial_response(
    http: &serenity::Http,
    interaction_id: serenity::InteractionId,
    token: &str,
    content: &str,
) -> Result<(), Error> {
    let payload = serde_json::json!({
        "type": 4, // CHANNEL_MESSAGE_WITH_SOURCE
        "data": {
            "flags": IS_COMPONENTS_V2,
            "components": text_display_components(content),
        },
    });
    http.create_interaction_response(interaction_id, token, &payload, vec![])
        .await?;
    Ok(())
}

/// Edit the original interaction response (`PATCH .../@original`). This is the only edit
/// endpoint that keeps custom emoji rendered (discord-api-docs#4431). Fired manually
/// because serenity would otherwise try to deserialize the type-10 component in the reply.
async fn edit_v2_original(
    http: &serenity::Http,
    application_id: serenity::ApplicationId,
    token: &str,
    content: &str,
) -> Result<(), Error> {
    let payload = serde_json::json!({
        "components": text_display_components(content),
    });
    let request = serenity::http::Request::new(
        serenity::http::Route::WebhookOriginalInteractionResponse {
            application_id,
            token,
        },
        serenity::http::LightMethod::Patch,
    )
    .body(Some(serde_json::to_vec(&payload)?));

    let _: serde_json::Value = http.fire(request).await?;
    Ok(())
}

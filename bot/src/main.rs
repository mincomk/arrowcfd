mod commands;
mod grid;

use poise::serenity_prelude as serenity;

pub struct Data {}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let mut solve = commands::solve();
    solve.install_context = Some(vec![
        serenity::InstallationContext::Guild,
        serenity::InstallationContext::User,
    ]);
    solve.interaction_context = Some(vec![
        serenity::InteractionContext::Guild,
        serenity::InteractionContext::BotDm,
        serenity::InteractionContext::PrivateChannel,
    ]);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![solve],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}

use crate::{Context, Error};
use poise::serenity_prelude as serenity;

#[poise::command(context_menu_command = "Solve")]
pub async fn solve(ctx: Context<'_>, msg: serenity::Message) -> Result<(), Error> {
    let grid = crate::grid::extract_grid(&msg.content);
    println!("{:?}", grid);
    ctx.say("Hello, world!").await?;
    Ok(())
}

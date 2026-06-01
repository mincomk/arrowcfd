//! One-shot tool: upload the arrow emojis listed in an `emojis.json`-format file
//! into this application's emoji store (named `arrow_0`..`arrow_359`), then write a
//! new file in the same format whose ids are the application-level emoji ids.
//!
//! Usage: `register_emojis <input.json> [output.json]`
//! (defaults: `emojis.json` -> `emojis.app.json`).
//!
//! Already-present names are skipped (their existing app id is reused in the output),
//! so it is safe to re-run. Needs `DISCORD_TOKEN` (read from `.env`).

use base64::Engine as _;
use regex::Regex;
use serenity::all as serenity;
use std::collections::HashMap;

type Error = Box<dyn std::error::Error + Send + Sync>;

const SIZE: u32 = 128;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");

    let mut args = std::env::args().skip(1);
    let input = args.next().unwrap_or_else(|| "emojis.json".to_string());
    let output = args.next().unwrap_or_else(|| "emojis.app.json".to_string());

    let http = serenity::Http::new(&token);
    let app = http.get_current_application_info().await?;
    http.set_application_id(app.id);
    println!("application: {} ({})", app.name, app.id);

    let mut existing: HashMap<String, serenity::Emoji> = http
        .get_application_emojis()
        .await?
        .into_iter()
        .map(|e| (e.name.clone(), e))
        .collect();

    let client = reqwest::Client::new();
    let re = Regex::new(r#""arrow_(\d+)"\s*:\s*"(a?\d+)""#).unwrap();
    let raw = std::fs::read_to_string(&input)?;

    let mut created = 0usize;
    let mut skipped = 0usize;
    let mut entries: Vec<(u32, String)> = Vec::new();

    for caps in re.captures_iter(&raw) {
        let deg: u32 = caps[1].parse().unwrap();
        let name = format!("arrow_{deg}");

        // A leading `a` on the source id marks an animated emoji.
        let (animated, id) = match caps[2].strip_prefix('a') {
            Some(id) => (true, id),
            None => (false, &caps[2]),
        };

        let emoji = if let Some(emoji) = existing.get(&name) {
            skipped += 1;
            emoji.clone()
        } else {
            let (ext, mime) = if animated {
                ("gif", "image/gif")
            } else {
                ("png", "image/png")
            };
            let url = format!("https://cdn.discordapp.com/emojis/{id}.{ext}?size={SIZE}");
            match upload(&client, &http, &url, mime, &name).await {
                Ok(emoji) => {
                    created += 1;
                    println!("created {name} -> {}", emoji.id);
                    existing.insert(name.clone(), emoji.clone());
                    emoji
                }
                Err(err) => {
                    eprintln!("failed {name} ({id}): {err}");
                    continue;
                }
            }
        };

        let value = if emoji.animated {
            format!("a{}", emoji.id)
        } else {
            emoji.id.to_string()
        };
        entries.push((deg, value));
    }

    entries.sort_by_key(|(deg, _)| *deg);
    let body = entries
        .iter()
        .map(|(deg, value)| format!("  \"arrow_{deg}\": \"{value}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    std::fs::write(&output, format!("{{\n{body}\n}}\n"))?;

    println!(
        "done: {created} created, {skipped} already present; wrote {} entries to {output}",
        entries.len()
    );
    Ok(())
}

async fn upload(
    client: &reqwest::Client,
    http: &serenity::Http,
    url: &str,
    mime: &str,
    name: &str,
) -> Result<serenity::Emoji, Error> {
    let bytes = client.get(url).send().await?.error_for_status()?.bytes().await?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let image = format!("data:{mime};base64,{encoded}");
    let body = serde_json::json!({ "name": name, "image": image });
    Ok(http.create_application_emoji(&body).await?)
}

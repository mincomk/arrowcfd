use crate::grid::Cell;
use base64::Engine as _;
use poise::serenity_prelude as serenity;
use std::collections::{HashMap, HashSet};

/// Markup for a custom emoji reference: `<a:name:id>` if animated, else `<:name:id>`.
fn markup(animated: bool, name: &str, id: serenity::EmojiId) -> String {
    if animated {
        format!("<a:{name}:{id}>")
    } else {
        format!("<:{name}:{id}>")
    }
}

pub async fn resolve_solids(
    http: &serenity::Http,
    cells: &[Vec<Cell>],
    size: u32,
) -> HashMap<u64, String> {
    let mut wanted: HashMap<u64, bool> = HashMap::new();
    for row in cells {
        for cell in row {
            if let Cell::Emoji { id, animated } = *cell {
                wanted.entry(id).or_insert(animated);
            }
        }
    }

    let mut out = HashMap::new();
    if wanted.is_empty() {
        return out;
    }

    let existing: HashMap<String, serenity::model::guild::Emoji> = match http
        .get_application_emojis()
        .await
    {
        Ok(emojis) => emojis.into_iter().map(|e| (e.name.clone(), e)).collect(),
        Err(err) => {
            eprintln!("failed to list application emojis: {err}");
            HashMap::new()
        }
    };

    let client = reqwest::Client::new();
    let mut seen_names: HashSet<String> = HashSet::new();

    for (id, animated) in wanted {
        let name = format!("e{id}");
        seen_names.insert(name.clone());

        if let Some(existing) = existing.get(&name) {
            out.insert(id, markup(existing.animated, &existing.name, existing.id));
            continue;
        }

        match upload(&client, http, id, animated, &name, size).await {
            Ok(emoji) => {
                out.insert(id, markup(emoji.animated, &emoji.name, emoji.id));
            }
            Err(err) => eprintln!("failed to import emoji {id}: {err}"),
        }
    }

    out
}

async fn upload(
    client: &reqwest::Client,
    http: &serenity::Http,
    id: u64,
    animated: bool,
    name: &str,
    size: u32,
) -> Result<serenity::model::guild::Emoji, crate::Error> {
    let (ext, mime) = if animated {
        ("gif", "image/gif")
    } else {
        ("png", "image/png")
    };
    let url = format!("https://cdn.discordapp.com/emojis/{id}.{ext}?size={size}");

    let bytes = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let image = format!("data:{mime};base64,{encoded}");

    let body = serde_json::json!({ "name": name, "image": image });
    let emoji = http.create_application_emoji(&body).await?;
    Ok(emoji)
}

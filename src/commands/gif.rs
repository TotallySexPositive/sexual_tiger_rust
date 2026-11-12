use crate::myauth::AuthData;

use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
struct GiphyUser {}

#[derive(Debug, Serialize, Deserialize)]
struct GiphyImages {}

#[derive(Debug, Serialize, Deserialize)]
struct GiphyGifObject {
    r#type: Option<String>,
    id: Option<String>,
    slug: Option<String>,
    url: Option<String>,
    bitly_url: Option<String>,
    embed_url: Option<String>,
    username: Option<String>,
    source: Option<String>,
    rating: Option<String>,
    content_url: Option<String>,
    user: Option<GiphyUser>,
    source_tld: Option<String>,
    source_post_url: Option<String>,
    update_datetime: Option<String>,
    create_datetime: Option<String>,
    import_datetime: Option<String>,
    trending_datetime: Option<String>,
    images: Option<GiphyImages>,
    title: Option<String>,
    alt_text: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct GiphyMetaObject {}
#[derive(Debug, Serialize, Deserialize)]
struct GiphyPayload {
    data: GiphyGifObject,
    meta: GiphyMetaObject,
}

#[command]
pub async fn gif(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let api_key = &data.get::<AuthData>().unwrap().giphy;

    if api_key.is_empty() {
        msg.reply(
            ctx,
            "Giphy token is not configured. Set GIPHY_TOKEN in your environment or auth.json to use ~gif.",
        )
        .await?;
        return Ok(());
    }

    let query = args.rest();
    match fetch_random_gif(api_key, query).await {
        Ok(url) => {
            msg.reply(ctx, url).await?;
        }
        Err(e) => {
            msg.reply(ctx, format!("Error fetching gif: {}", e)).await?;
        }
    }
    Ok(())
}

pub async fn fetch_random_gif(api_key: &str, query: &str) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("Giphy token is not configured.".to_string());
    }
    let res = reqwest::get(&format!(
        "https://api.giphy.com/v1/gifs/random?api_key={}&tag={}&rating=r",
        api_key, query
    ))
    .await
    .map_err(|e| format!("Failed to reach Giphy: {}", e))?;

    let body = res
        .text()
        .await
        .map_err(|e| format!("Failed to read Giphy response: {}", e))?;
    let payload = serde_json::from_str::<GiphyPayload>(&body)
        .map_err(|e| format!("Failed to parse Giphy json: {}", e))?;
    payload
        .data
        .embed_url
        .ok_or_else(|| "Giphy response had no embed url".to_string())
}

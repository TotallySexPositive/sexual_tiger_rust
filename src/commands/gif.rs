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

    let query = args.rest();
    let res = reqwest::get(&format!(
        "https://api.giphy.com/v1/gifs/random?api_key={}&tag={}&rating=r",
        api_key, query
    ))
    .await
    .unwrap();
    let body = res.text().await.unwrap();
    let payload = serde_json::from_str::<GiphyPayload>(&body)?;
    let embed_url = payload
        .data
        .embed_url
        .expect("payload should have embed url");
    msg.reply(ctx, embed_url).await?;
    Ok(())
}

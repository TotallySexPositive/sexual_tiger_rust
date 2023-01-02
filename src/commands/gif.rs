use crate::myauth::AuthData;

use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;

#[command]
pub async fn gif(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let api_key = &data.get::<AuthData>().unwrap().giphy;

    let query = args.rest();
    let res = reqwest::get(&format!(
        "https://api.giphy.com/v1/gifs/random?api_key={}&tag={}&rating=r",
        api_key, query
    ))
    .await?;
    let body = res.text().await?;
    let json: serde_json::Value = serde_json::from_str(&body)?;
    let embed_url = json["data"]["embed_url"].as_str().unwrap();
    msg.reply(ctx, embed_url).await?;
    Ok(())
}

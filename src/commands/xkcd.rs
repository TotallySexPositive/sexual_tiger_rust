use core::fmt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::all::ReactionType;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::colour::Color;
use serenity::model::prelude::*;
use std::time::Duration;

#[derive(Debug)]
enum XkcdError {
    JsonError,
    HttpError,
    SendError,
}

impl std::error::Error for XkcdError {}

impl fmt::Display for XkcdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use XkcdError::*;
        match self {
            JsonError => write!(f, "Xkcd::JsonError"),
            HttpError => write!(f, "Xkcd::HttpError"),
            SendError => write!(f, "Xkcd::SendError"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct XkcdPayload {
    month: String,
    num: i64,
    link: String,
    year: String,
    news: String,
    safe_title: String,
    transcript: String,
    alt: String,
    img: String,
    title: String,
    day: String,
}

fn random_button() -> CreateButton {
    let emo: ReactionType = "❓".parse().unwrap();
    CreateButton::new("random").emoji(emo)
}

async fn random_comic(max: i64) -> Result<CreateEmbed, XkcdError> {
    let num = rand::thread_rng().gen_range(1..max);
    comic_content(num).await
}

async fn any_comic(num: i64) -> Result<XkcdPayload, XkcdError> {
    let latest = match reqwest::get(format!("https://xkcd.com/{}/info.0.json", num)).await {
        Ok(p) => p,
        Err(_) => return Err(XkcdError::HttpError),
    };
    let body = match latest.text().await {
        Ok(p) => p,
        Err(_) => return Err(XkcdError::HttpError),
    };
    match serde_json::from_str::<XkcdPayload>(&body) {
        Ok(p) => Ok(p),
        Err(_) => Err(XkcdError::JsonError),
    }
}

async fn new_comic() -> XkcdPayload {
    let latest = reqwest::get("https://xkcd.com/info.0.json").await.unwrap();
    let body = latest.text().await.unwrap();
    serde_json::from_str(&body).unwrap()
}

async fn comic_content(num: i64) -> Result<CreateEmbed, XkcdError> {
    let payload = any_comic(num).await?;
    println!("{:?}", &serde_json::to_string_pretty(&payload));
    let title = payload.title;
    let embed_url = payload.img;
    let alt_text = payload.alt;

    let embed = CreateEmbed::new()
        .image(embed_url)
        .color(Color::GOLD)
        .title(title)
        .footer(CreateEmbedFooter::new(alt_text));

    Ok(embed)
}

async fn react(ctx: &Context, msg: &Message, max: i64) -> Result<(), XkcdError> {
    if let Some(interaction) = msg
        .await_component_interaction(&ctx.shard)
        .timeout(Duration::from_secs(10))
        .await
    {
        println!("Reaction!");
        let new_content = random_comic(max).await?;
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(new_content),
        );
        interaction
            .create_response(&ctx.http, response)
            .await
            .map_err(|_| XkcdError::SendError)?;
    } else {
        println!("No reaction :()");
        let _ = msg.reply(&ctx.http, "No reaction within 10 sec.").await;
    }
    Ok(())
}

async fn respond(ctx: &Context, msg: &Message, max: i64) -> Result<Message, XkcdError> {
    let content = random_comic(max).await?;

    let builder = CreateMessage::new()
        .content("Do you want another random comic?")
        .embed(content)
        .components(vec![CreateActionRow::Buttons(vec![random_button()])]);

    msg.channel_id
        .send_message(&ctx.http, builder)
        .await
        .map_err(|_| XkcdError::SendError)
}

#[command]
pub async fn xkcd(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let max = new_comic().await.num;

    let m = respond(ctx, msg, max).await?;
    react(ctx, &m, max).await?;

    Ok(())
}

pub async fn get_random_xkcd() -> Result<CreateEmbed, String> {
    let latest = reqwest::get("https://xkcd.com/info.0.json")
        .await
        .map_err(|e| format!("Failed to reach xkcd: {}", e))?;
    let body = latest
        .text()
        .await
        .map_err(|e| format!("Failed to read xkcd response: {}", e))?;
    let payload: XkcdPayload =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse xkcd: {}", e))?;
    let max = payload.num;
    let num = rand::thread_rng().gen_range(1..max);
    comic_content(num)
        .await
        .map_err(|e| format!("Failed to load comic: {:?}", e))
}

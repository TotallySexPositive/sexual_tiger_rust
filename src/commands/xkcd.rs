use core::fmt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::builder::{CreateButton, CreateEmbed};
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
//use serenity::prelude::*;
use serenity::utils::Color;
use std::time::Duration;

#[derive(Debug)]
enum XkcdError {
    JsonError,
    HttpError,
    SendError,
    Unknown,
}
impl std::error::Error for XkcdError {}
impl fmt::Display for XkcdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use XkcdError::*;
        match self {
            JsonError => write!(f, "Xkcd::JsonError"),
            HttpError => write!(f, "Xkcd::HttpError"),
            SendError => write!(f, "Xkcd::SendError"),
            Unknown => write!(f, "Xkcd::Unknown"),
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
    let mut b = CreateButton::default();
    let emo: ReactionType = "❓".parse().unwrap();
    b.custom_id("random");
    b.emoji(emo);
    b
}

async fn random_comic(max: i64) -> Result<CreateEmbed, XkcdError> {
    let x = comic_content(rand::thread_rng().gen_range(1..max));
    x.await
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
        Ok(p) => return Ok(p),
        Err(_) => return Err(XkcdError::JsonError),
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
    let _num = payload.num;
    let mut embed = CreateEmbed::default();
    embed.image(embed_url);
    embed.color(Color::GOLD);
    embed.title(title);
    embed.footer(|f| f.text(alt_text));
    Ok(embed)
}

async fn react(ctx: &Context, msg: &Message, max: i64) -> Result<(), XkcdError> {
    if let Some(reaction) = &msg
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(10))
        .await
    {
        println!("Reaction!");
        let new_content = random_comic(max).await?;
        reaction
            .create_interaction_response(ctx, |r| {
                r.kind(interaction::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|f| f.set_embed(new_content))
            })
            .await
            .unwrap();
    } else {
        println!("No reaction :()");
        let _ = msg.reply(ctx, "No reaction within 10 sec.");
    }
    Ok(())
}

async fn respond(ctx: &Context, msg: &Message, max: i64) -> Result<Message, XkcdError> {
    let content = random_comic(max).await?;

    match msg
        .channel_id
        .send_message(&ctx, |m| {
            m.content("Do you want another random comic?")
                .set_embed(content)
                .components(|c| c.create_action_row(|row| row.add_button(random_button())))
        })
        .await
    {
        Ok(p) => Ok(p),
        Err(_) => return Err(XkcdError::SendError),
    }
}

#[command]
pub async fn xkcd(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let max = new_comic().await.num;

    let m = respond(ctx, msg, max).await?;
    react(ctx, &m, max).await?;

    Ok(())
}

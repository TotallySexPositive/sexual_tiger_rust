use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::builder::{CreateButton, CreateEmbed};
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::utils::Color;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
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

async fn random_comic(max: i64) -> CreateEmbed {
    let x = comic_content(rand::thread_rng().gen_range(1..max));
    x.await
}

async fn any_comic(num: i64) -> XkcdPayload {
    let latest = reqwest::get(format!("https://xkcd.com/{}/info.0.json", num))
        .await
        .unwrap();
    let body = latest.text().await.unwrap();
    serde_json::from_str(&body).unwrap()
}

async fn new_comic() -> XkcdPayload {
    let latest = reqwest::get("https://xkcd.com/info.0.json").await.unwrap();
    let body = latest.text().await.unwrap();
    serde_json::from_str(&body).unwrap()
}

async fn comic_content(num: i64) -> CreateEmbed {
    let payload = any_comic(num).await;
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    let title = payload.title;
    let embed_url = payload.img;
    let alt_text = payload.alt;
    let _num = payload.num;
    let mut embed = CreateEmbed::default();
    embed.image(embed_url);
    embed.color(Color::GOLD);
    embed.title(title);
    embed.footer(|f| f.text(alt_text));
    embed
}

#[command]
pub async fn xkcd(ctx: &Context, msg: &Message) -> CommandResult {
    let content = random_comic(1000).await;

    let m: serenity::model::channel::Message = msg
        .channel_id
        .send_message(&ctx, |m| {
            m.content("Do you want another random comic?")
                .set_embed(content)
                .components(|c| c.create_action_row(|row| row.add_button(random_button())))
        })
        .await
        .unwrap();

    /*    if let Some(reaction) = &m
            .await_component_interaction(ctx)
            .timeout(Duration::from_secs(10))
            .await
        {
            Some(reaction.channel_id.send_message(ctx, comic_content(10)));
        }
    */
    Ok(())
}

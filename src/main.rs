use std::env;

use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{Args, CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;

#[group]
#[commands(ping, gif)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) //set the bot's prefix to "~"
        .group(&GENERAL_GROUP);
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;
    Ok(())
}

#[command]
async fn gif(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let api_key = env::var("GIPHY_TOKEN").expect("Expected GIPHY_TOKEN in env");
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

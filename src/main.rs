#![allow(deprecated)]

mod commands;
mod myauth;

use std::env;

use crate::commands::download::*;
use crate::commands::fuckoff::*;
use crate::commands::gif::*;
use crate::commands::ping::*;
use crate::commands::play::*;
use crate::commands::playlist::*;
use crate::commands::xkcd::*;
use myauth::AuthData;
use sexual_tiger::{establish_connection_pool, DatabasePool};

use serenity::async_trait;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::Configuration;
use serenity::framework::standard::StandardFramework;
use serenity::prelude::*;

use songbird::SerenityInit;

#[group]
#[commands(
    ping,
    gif,
    xkcd,
    play,
    skip,
    stop,
    pause,
    resume,
    queue,
    songs,
    join,
    leave,
    mute,
    unmute,
    deafen,
    undeafen,
    download,
    fuckoff,
    playlist,
    playplaylist
)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix("~"));

    // Configure the client with your Discord bot token in the environment.
    let discord = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let giphy = env::var("GIPHY_TOKEN").expect("Expected GIPHY_TOKEN in env");
    let auth = AuthData { giphy, discord };

    let pool = establish_connection_pool().await;

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&auth.discord, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<AuthData>(auth);
        data.insert::<DatabasePool>(pool);
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

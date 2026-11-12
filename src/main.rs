#![allow(deprecated)]

mod commands;
mod myauth;
mod slash;

use crate::commands::download::*;
use crate::commands::fuckoff::*;
use crate::commands::gif::*;
use crate::commands::help::*;
use crate::commands::ping::*;
use crate::commands::play::*;
use crate::commands::playlist::*;
use crate::commands::volume::*;
use crate::commands::xkcd::*;
use myauth::AuthData;
use sexual_tiger::{establish_connection_pool, DatabasePool};

use serenity::async_trait;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::Configuration;
use serenity::framework::standard::StandardFramework;
use serenity::model::application::{Command, Interaction};
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use songbird::SerenityInit;

#[group]
#[commands(
    help,
    ping,
    gif,
    xkcd,
    play,
    volume,
    vup,
    vdown,
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
    playplaylist,
    shuffleplaylist,
    shuffle,
    wii
)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let slash_cmds = slash::create_slash_commands();
        match Command::set_global_commands(&ctx.http, slash_cmds).await {
            Ok(cmds) => {
                println!(
                    "Successfully registered {} global slash commands with Discord!",
                    cmds.len()
                );
            }
            Err(why) => {
                eprintln!("Failed to register global slash commands: {:?}", why);
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            slash::handle_slash_command(&ctx, &command).await;
        }
    }
}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix("~"));

    // Load authentication credentials from environment variables, .env, or auth.json/configure.json
    let auth = AuthData::load().unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    });

    let pool = establish_connection_pool().await;

    // Start background Web API server for DataTables / data.json / /api/songs
    let api_pool = pool.clone();
    let api_port = std::env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let api_host = std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    if let Err(e) = sexual_tiger::export_data_json(&pool).await {
        eprintln!("Initial data.json export notice: {}", e);
    }

    tokio::spawn(async move {
        if let Err(e) = sexual_tiger::start_api_server(api_pool, &api_host, api_port).await {
            eprintln!("API server error: {}", e);
        }
    });

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

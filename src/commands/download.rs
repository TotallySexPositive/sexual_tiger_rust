use sexual_tiger::{download_song, insert_song, new_song, DatabasePool};

use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::channel::Message, Result as SerenityResult};

#[command]
pub async fn download(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let uri = match args.single::<String>() {
        Ok(x) => x,
        Err(_) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Must provide a URL to video or audio")
                    .await,
            );
            return Ok(());
        }
    };

    let song_dir = sexual_tiger::get_song_directory();

    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabasePool>()
            .cloned()
            .expect("Expected DatabasePool in TypeMap")
    };

    check_msg(
        msg.channel_id
            .say(&ctx.http, "Downloading audio, please wait...")
            .await,
    );

    let meta = match download_song(uri).await {
        Ok(m) => m,
        Err(e) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed to download song: {}", e))
                    .await,
            );
            return Ok(());
        }
    };

    let title = meta.title.clone().unwrap_or_else(|| "Unknown".to_string());
    let source_path = format!("{}/{}.webm", song_dir.trim_end_matches('/'), &title);
    let new_s = new_song(&meta, msg.author.name.clone(), source_path);

    match insert_song(&pool, &new_s).await {
        Ok(s) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Successfully added song: {}", s.name))
                    .await,
            );
        }
        Err(e) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Error saving song to database: {}", e))
                    .await,
            );
        }
    }

    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

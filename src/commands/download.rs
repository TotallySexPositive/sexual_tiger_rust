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

    let downloaded = match download_song(uri).await {
        Ok(d) => d,
        Err(e) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed to download song: {}", e))
                    .await,
            );
            return Ok(());
        }
    };

    let new_s = match new_song(&downloaded.metadata, msg.author.name.clone(), downloaded.source_path) {
        Ok(s) => s,
        Err(e) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed to process downloaded song: {}", e))
                    .await,
            );
            return Ok(());
        }
    };

    match insert_song(&pool, &new_s).await {
        Ok(s) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Successfully added song: {}", s.name))
                    .await,
            );
        }
        Err(e) => {
            let err_str = e.to_string();
            let msg_text = if err_str.contains("UNIQUE constraint failed") {
                format!("Song **{}** is already in the library!", new_s.name)
            } else {
                format!("Error saving song to database: {}", e)
            };
            check_msg(
                msg.channel_id
                    .say(&ctx.http, msg_text)
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

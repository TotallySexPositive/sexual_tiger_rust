use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

#[command]
pub async fn fuckoff(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            let _ = msg.reply(&ctx.http, "Must be in a server").await;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        {
            let mut handler = handler_lock.lock().await;
            handler.stop();
            handler.queue().stop();
        }
        let _ = manager.remove(guild_id).await;
        let _ = msg.channel_id.say(&ctx.http, "Left voice channel. Bye!").await;
    } else {
        let _ = msg.reply(&ctx.http, "Not in a voice channel").await;
    }

    Ok(())
}

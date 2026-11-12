pub mod download;
pub mod fuckoff;
pub mod gif;
pub mod help;
pub mod ping;
pub mod play;
pub mod playlist;
pub mod volume;
pub mod xkcd;

use serenity::client::Context;
use serenity::model::id::ChannelId;
use sexual_tiger::chunk_text;

pub async fn send_chunked(ctx: &Context, channel_id: ChannelId, text: &str) {
    for chunk in chunk_text(text, 1900) {
        if !chunk.is_empty() {
            if let Err(why) = channel_id.say(&ctx.http, chunk).await {
                eprintln!("Error sending chunked message: {:?}", why);
            }
        }
    }
}

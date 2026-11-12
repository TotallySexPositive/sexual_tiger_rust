pub mod download;
pub mod fuckoff;
pub mod gif;
pub mod help;
pub mod ping;
pub mod play;
pub mod playlist;
pub mod volume;
pub mod xkcd;

use serenity::builder::{CreateAllowedMentions, CreateMessage};
use serenity::client::Context;
use serenity::model::id::ChannelId;
use sexual_tiger::chunk_text;

pub async fn ensure_voice_channel(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    user_id: serenity::model::id::UserId,
) -> Result<std::sync::Arc<tokio::sync::Mutex<songbird::Call>>, String> {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        return Ok(handler_lock);
    }

    let channel_id = ctx
        .cache
        .guild(guild_id)
        .and_then(|g| g.voice_states.get(&user_id).and_then(|vs| vs.channel_id));

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => return Err("You must be in a voice channel to play music!".to_string()),
    };

    let _ = manager.join(guild_id, connect_to).await;

    match manager.get(guild_id) {
        Some(h) => Ok(h),
        None => Err("Failed to join voice channel.".to_string()),
    }
}

pub async fn send_chunked(ctx: &Context, channel_id: ChannelId, text: &str) {
    for chunk in chunk_text(text, 1900) {
        if !chunk.is_empty() {
            let builder = CreateMessage::new()
                .content(chunk)
                .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles());
            if let Err(why) = channel_id.send_message(&ctx.http, builder).await {
                eprintln!("Error sending chunked message: {:?}", why);
            }
        }
    }
}

pub async fn resolve_creator_name(http: &serenity::http::Http, created_by: &str) -> String {
    let trimmed = created_by.trim();
    if let Ok(uid) = trimmed.parse::<u64>() {
        let user_id = serenity::model::id::UserId::new(uid);
        if let Ok(user) = user_id.to_user(http).await {
            return user.global_name.unwrap_or(user.name);
        }
        return format!("User {}", uid);
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_creator_name_username() {
        let http = serenity::http::Http::new("");
        let name = resolve_creator_name(&http, "Papa").await;
        assert_eq!(name, "Papa");
    }

    #[tokio::test]
    async fn test_resolve_creator_name_id_fallback_no_ping() {
        let http = serenity::http::Http::new("");
        // Without valid token, fails to fetch from Discord API and falls back to plain text without pings
        let name = resolve_creator_name(&http, "231574835694796801").await;
        assert_eq!(name, "User 231574835694796801");
        assert!(!name.contains('<'), "Must never contain Discord mention brackets");
    }
}

use serenity::async_trait;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
use sexual_tiger::{
    find_song_by_query, list_songs, record_song_play, search_songs, DatabasePool,
};
use songbird::events::{Event, EventContext, EventHandler, TrackEvent};

struct TrackErrorNotifier;

#[async_trait]
impl EventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                eprintln!(
                    "Audio track {:?} encountered error. Position: {:?}, Error: {:?}",
                    handle.uuid(),
                    state.position,
                    state.playing
                );
            }
        }
        None
    }
}

#[command]
pub async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let in_query = match args.remains() {
        Some(q) if !q.trim().is_empty() => q.trim().to_string(),
        _ => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Must provide a song name or a URL to audio/video")
                    .await,
            );
            return Ok(());
        }
    };

    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let handler_lock = match crate::commands::ensure_voice_channel(ctx, guild_id, msg.author.id).await {
        Ok(h) => h,
        Err(e) => {
            check_msg(msg.channel_id.say(&ctx.http, e).await);
            return Ok(());
        }
    };

    // If it's not a URL, look it up in the database
    if !in_query.starts_with("http://") && !in_query.starts_with("https://") {
        let pool = {
            let data = ctx.data.read().await;
            data.get::<DatabasePool>()
                .cloned()
                .expect("Expected DatabasePool in TypeMap")
        };

        let maybe_song = match find_song_by_query(&pool, &in_query).await {
            Ok(s) => s,
            Err(e) => {
                check_msg(
                    msg.channel_id
                        .say(&ctx.http, format!("Database error: {}", e))
                        .await,
                );
                return Ok(());
            }
        };

        let the_song = match maybe_song {
            Some(s) => s,
            None => {
                check_msg(
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!("Song not found matching: \"{}\". Use `~songs` to see available songs.", in_query),
                        )
                        .await,
                );
                return Ok(());
            }
        };

        println!("Playing local song: {:?}", the_song);

        let resolved_source = sexual_tiger::resolve_song_path(&the_song.source);
        if !std::path::Path::new(&resolved_source).exists() {
            check_msg(
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("Warning: Audio file not found on disk at `{}`. Please check mounted audio volume.", resolved_source),
                    )
                    .await,
            );
        }

        let input: songbird::input::Input =
            songbird::input::File::new(resolved_source).into();

        {
            let mut handler = handler_lock.lock().await;

            let track = handler.enqueue_input(input).await;
            let _ = track.add_event(Event::Track(TrackEvent::Error), TrackErrorNotifier);
            if the_song.is_clip.unwrap_or(0) == 1 {
                let _ = track.set_volume(5.0);
            } else {
                let _ = track.set_volume(1.5);
            }

            let q_len = handler.queue().len();
            let id_info = the_song
                .song_id
                .map(|id| format!(" (ID: {})", id))
                .unwrap_or_default();
            if q_len > 1 {
                check_msg(
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!("Queued **{}**{} (position: {})", the_song.name, id_info, q_len),
                        )
                        .await,
                );
            } else {
                check_msg(
                    msg.channel_id
                        .say(&ctx.http, format!("Playing **{}**{}", the_song.name, id_info))
                        .await,
                );
            }
        }

        if let Some(sid) = the_song.song_id {
            let current_plays = the_song.num_plays.unwrap_or(0);
            if let Err(e) = record_song_play(&pool, sid, current_plays).await {
                println!("Error updating play count: {}", e);
            }
        }
        return Ok(());
    }

    // It's a web URL
    {
        let mut handler = handler_lock.lock().await;

        let clean_url = sexual_tiger::clean_youtube_url(&in_query);
        let input: songbird::input::Input =
            songbird::input::YoutubeDl::new(reqwest::Client::new(), clean_url).into();

        let track = handler.enqueue_input(input).await;
        let _ = track.add_event(Event::Track(TrackEvent::Error), TrackErrorNotifier);
        let q_len = handler.queue().len();
        if q_len > 1 {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Queued stream (position: {})", q_len))
                    .await,
            );
        } else {
            check_msg(msg.channel_id.say(&ctx.http, "Playing stream").await);
        }
    }
    Ok(())
}

#[command]
pub async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.is_empty() {
            check_msg(msg.channel_id.say(&ctx.http, "Queue is empty.").await);
        } else {
            let _ = queue.skip();
            check_msg(msg.channel_id.say(&ctx.http, "Skipped song.").await);
        }
    } else {
        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
pub async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.stop();
        handler.queue().stop();
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Stopped playing and cleared the queue.")
                .await,
        );
    } else {
        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
pub async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        if let Err(e) = handler.queue().pause() {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed to pause: {:?}", e))
                    .await,
            );
        } else {
            check_msg(msg.channel_id.say(&ctx.http, "Playback paused.").await);
        }
    } else {
        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
pub async fn resume(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        if let Err(e) = handler.queue().resume() {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed to resume: {:?}", e))
                    .await,
            );
        } else {
            check_msg(msg.channel_id.say(&ctx.http, "Playback resumed.").await);
        }
    } else {
        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
pub async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let q_len = handler.queue().len();
        if q_len == 0 {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "The queue is currently empty.")
                    .await,
            );
        } else {
            check_msg(
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("There are **{}** song(s) in the queue.", q_len),
                    )
                    .await,
            );
        }
    } else {
        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
pub async fn songs(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabasePool>()
            .cloned()
            .expect("Expected DatabasePool in TypeMap")
    };

    let search_query = args.remains().map(|q| q.trim()).unwrap_or("");

    let song_list = if search_query.is_empty() {
        match list_songs(&pool, 100).await {
            Ok(s) => s,
            Err(e) => {
                check_msg(
                    msg.channel_id
                        .say(&ctx.http, format!("Database error: {}", e))
                        .await,
                );
                return Ok(());
            }
        }
    } else {
        match search_songs(&pool, search_query).await {
            Ok(s) => s,
            Err(e) => {
                check_msg(
                    msg.channel_id
                        .say(&ctx.http, format!("Database error: {}", e))
                        .await,
                );
                return Ok(());
            }
        }
    };

    if song_list.is_empty() {
        if search_query.is_empty() {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "No songs in the database. Download some with `~download <url>`!")
                    .await,
            );
        } else {
            check_msg(
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("No songs found matching \"{}\".", search_query),
                    )
                    .await,
            );
        }
        return Ok(());
    }

    let title_prefix = if search_query.is_empty() {
        "Top Played Songs".to_string()
    } else {
        format!("Search results for \"{}\"", search_query)
    };

    let pages = sexual_tiger::format_songs_pages(&title_prefix, &song_list);
    for page in pages {
        check_msg(msg.channel_id.say(&ctx.http, page).await);
    }
    Ok(())
}

#[command]
pub async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let channel_id = ctx
        .cache
        .guild(guild_id)
        .and_then(|g| g.voice_states.get(&msg.author.id).and_then(|vs| vs.channel_id));

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let _ = manager.join(guild_id, connect_to).await;

    Ok(())
}

#[command]
pub async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Left voice channel").await);
    } else {
        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
pub async fn mute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        check_msg(msg.channel_id.say(&ctx.http, "Already muted").await);
    } else {
        if let Err(e) = handler.mute(true).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Now muted").await);
    }

    Ok(())
}

#[command]
pub async fn deafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        check_msg(msg.channel_id.say(&ctx.http, "Already deafened").await);
    } else {
        if let Err(e) = handler.deafen(true).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Deafened").await);
    }

    Ok(())
}

#[command]
pub async fn undeafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.deafen(false).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Undeafened").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to undeafen in")
                .await,
        );
    }

    Ok(())
}

#[command]
pub async fn unmute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.mute(false).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Unmuted").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to unmute in")
                .await,
        );
    }

    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

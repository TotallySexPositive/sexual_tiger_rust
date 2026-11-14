use serenity::async_trait;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
use sexual_tiger::{
    create_song_track, find_song_by_query, format_now_playing_message, get_track_user_data,
    list_songs, record_song_play, search_songs, DatabasePool, SongTrackInfo,
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

        play_song_struct(ctx, msg.channel_id, &handler_lock, &pool, &the_song).await;
        return Ok(());
    }

    // It's a web URL
    {
        let mut handler = handler_lock.lock().await;

        let clean_url = sexual_tiger::clean_youtube_url(&in_query);
        let input: songbird::input::Input =
            songbird::input::YoutubeDl::new(reqwest::Client::new(), clean_url.clone()).into();

        let track_info = SongTrackInfo {
            title: clean_url,
            duration_secs: None,
            song_id: None,
        };
        let track = handler.enqueue(create_song_track(input, track_info)).await;
        let _ = track.add_event(Event::Track(TrackEvent::Error), TrackErrorNotifier);
        let _ = track.set_volume(sexual_tiger::DEFAULT_BOT_VOLUME);
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
            if let Some(current) = queue.current() {
                let _ = current.disable_loop();
            }
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
        let queue = handler.queue();
        let q_len = queue.len();
        if q_len == 0 {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "The queue is currently empty.")
                    .await,
            );
        } else {
            let loop_tag = if let Some(current) = queue.current() {
                if let Ok(info) = current.get_info().await {
                    if matches!(info.loops, songbird::tracks::LoopState::Infinite) {
                        " (Current song is repeating 🔁)"
                    } else {
                        ""
                    }
                } else {
                    ""
                }
            } else {
                ""
            };
            check_msg(
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("There are **{}** song(s) in the queue.{}", q_len, loop_tag),
                    )
                    .await,
            );
        }
    } else {
        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatAction {
    Set(bool),
    Status,
}

pub fn parse_repeat_input(input: Option<&str>, currently_looping: bool) -> RepeatAction {
    match input.map(|s| s.trim().to_lowercase()) {
        None => RepeatAction::Set(!currently_looping),
        Some(s) if s.is_empty() => RepeatAction::Set(!currently_looping),
        Some(s) => match s.as_str() {
            "on" | "true" | "enable" | "yes" | "1" => RepeatAction::Set(true),
            "off" | "false" | "disable" | "no" | "0" => RepeatAction::Set(false),
            "toggle" => RepeatAction::Set(!currently_looping),
            "status" | "check" | "info" => RepeatAction::Status,
            _ => RepeatAction::Set(!currently_looping),
        },
    }
}

#[command]
#[aliases("loop")]
pub async fn repeat(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
        let current_track = match queue.current() {
            Some(t) => t,
            None => {
                check_msg(
                    msg.channel_id
                        .say(&ctx.http, "Nothing is currently playing to repeat.")
                        .await,
                );
                return Ok(());
            }
        };

        let info = current_track.get_info().await;
        let currently_looping = info
            .as_ref()
            .map(|i| matches!(i.loops, songbird::tracks::LoopState::Infinite))
            .unwrap_or(false);

        let arg_str = args.single::<String>().ok();
        let action = parse_repeat_input(arg_str.as_deref(), currently_looping);

        match action {
            RepeatAction::Status => {
                let status_msg = if currently_looping {
                    "Repeat is currently **ON** (🔁). Repeating current track endlessly until turned off, skipped, or stopped."
                } else {
                    "Repeat is currently **OFF**."
                };
                check_msg(msg.channel_id.say(&ctx.http, status_msg).await);
            }
            RepeatAction::Set(true) => {
                let _ = current_track.enable_loop();
                check_msg(
                    msg.channel_id
                        .say(
                            &ctx.http,
                            "Repeat is now **ON** (🔁). The current song will repeat forever until turned off, skipped, or stopped.",
                        )
                        .await,
                );
            }
            RepeatAction::Set(false) => {
                let _ = current_track.disable_loop();
                check_msg(
                    msg.channel_id
                        .say(
                            &ctx.http,
                            "Repeat is now **OFF**. The current song will finish and not repeat.",
                        )
                        .await,
                );
            }
        }
    } else {
        check_msg(msg.reply(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
#[aliases("np", "nowplaying", "current")]
pub async fn song(ctx: &Context, msg: &Message) -> CommandResult {
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
        let current_track = match queue.current() {
            Some(t) => t,
            None => {
                check_msg(
                    msg.channel_id
                        .say(&ctx.http, "Nothing is currently playing.")
                        .await,
                );
                return Ok(());
            }
        };

        let info = current_track.get_info().await.ok();
        let position = info.as_ref().map(|i| i.position).unwrap_or_default();
        let is_looping = info
            .as_ref()
            .map(|i| matches!(i.loops, songbird::tracks::LoopState::Infinite))
            .unwrap_or(false);

        let user_data = get_track_user_data(&current_track);
        let title = user_data
            .as_ref()
            .map(|d| d.title.as_str())
            .unwrap_or("Unknown Track");
        let duration_secs = user_data.as_ref().and_then(|d| d.duration_secs);
        let song_id = user_data.as_ref().and_then(|d| d.song_id);

        let response = format_now_playing_message(
            title,
            song_id,
            position,
            duration_secs,
            is_looping,
        );

        check_msg(msg.channel_id.say(&ctx.http, response).await);
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

pub const WII_SONG_IDS: &[i64] = &[2058, 2059, 2117, 2328, 2329, 2331, 2332, 2333, 2334];

pub async fn play_song_struct(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    handler_lock: &std::sync::Arc<tokio::sync::Mutex<songbird::Call>>,
    pool: &sqlx::SqlitePool,
    the_song: &sexual_tiger::models::Song,
) {
    play_song_struct_internal(ctx, channel_id, handler_lock, pool, the_song, true).await;
}

pub async fn play_song_struct_internal(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    handler_lock: &std::sync::Arc<tokio::sync::Mutex<songbird::Call>>,
    pool: &sqlx::SqlitePool,
    the_song: &sexual_tiger::models::Song,
    notify: bool,
) {
    println!("Playing local song: {:?}", the_song);

    let resolved_source = sexual_tiger::resolve_song_path(&the_song.source);
    if !std::path::Path::new(&resolved_source).exists() {
        check_msg(
            channel_id
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

        let track_info = SongTrackInfo {
            title: the_song.name.clone(),
            duration_secs: the_song.duration.map(|d| d as u64),
            song_id: the_song.song_id,
        };
        let track = handler.enqueue(create_song_track(input, track_info)).await;
        let _ = track.add_event(Event::Track(TrackEvent::Error), TrackErrorNotifier);
        if the_song.is_clip.unwrap_or(0) == 1 {
            let _ = track.set_volume(sexual_tiger::CLIP_VOLUME);
        } else {
            let _ = track.set_volume(sexual_tiger::DEFAULT_BOT_VOLUME);
        }

        if notify {
            let q_len = handler.queue().len();
            let id_info = the_song
                .song_id
                .map(|id| format!(" (ID: {})", id))
                .unwrap_or_default();
            if q_len > 1 {
                check_msg(
                    channel_id
                        .say(
                            &ctx.http,
                            format!("Queued **{}**{} (position: {})", the_song.name, id_info, q_len),
                        )
                        .await,
                );
            } else {
                check_msg(
                    channel_id
                        .say(&ctx.http, format!("Playing **{}**{}", the_song.name, id_info))
                        .await,
                );
            }
        }
    }

    if let Some(sid) = the_song.song_id {
        let current_plays = the_song.num_plays.unwrap_or(0);
        if let Err(e) = record_song_play(pool, sid, current_plays).await {
            println!("Error updating play count: {}", e);
        }
    }
}

#[command]
#[aliases("mii")]
pub async fn wii(ctx: &Context, msg: &Message) -> CommandResult {
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

    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabasePool>()
            .cloned()
            .expect("Expected DatabasePool in TypeMap")
    };

    let chosen_id = {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        *WII_SONG_IDS.choose(&mut rng).expect("WII_SONG_IDS is non-empty")
    };

    let maybe_song = match find_song_by_query(&pool, &chosen_id.to_string()).await {
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
                        format!("Wii song ID **{}** not found in database.", chosen_id),
                    )
                    .await,
            );
            return Ok(());
        }
    };

    // Play without notifying chat which song is being played
    play_song_struct_internal(ctx, msg.channel_id, &handler_lock, &pool, &the_song, false).await;
    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::SliceRandom;

    #[test]
    fn test_wii_song_ids_list() {
        assert_eq!(WII_SONG_IDS.len(), 9);
        let expected = [2058, 2059, 2117, 2328, 2329, 2331, 2332, 2333, 2334];
        for id in &expected {
            assert!(WII_SONG_IDS.contains(id), "Expected WII_SONG_IDS to contain {}", id);
        }

        let mut rng = rand::thread_rng();
        for _ in 0..50 {
            let picked = WII_SONG_IDS.choose(&mut rng).unwrap();
            assert!(expected.contains(picked));
        }
    }

    #[test]
    fn test_parse_repeat_input() {
        assert_eq!(parse_repeat_input(None, false), RepeatAction::Set(true));
        assert_eq!(parse_repeat_input(None, true), RepeatAction::Set(false));
        assert_eq!(parse_repeat_input(Some(""), false), RepeatAction::Set(true));
        assert_eq!(parse_repeat_input(Some("on"), false), RepeatAction::Set(true));
        assert_eq!(parse_repeat_input(Some("enable"), false), RepeatAction::Set(true));
        assert_eq!(parse_repeat_input(Some("true"), false), RepeatAction::Set(true));
        assert_eq!(parse_repeat_input(Some("yes"), false), RepeatAction::Set(true));
        assert_eq!(parse_repeat_input(Some("1"), false), RepeatAction::Set(true));
        assert_eq!(parse_repeat_input(Some("off"), true), RepeatAction::Set(false));
        assert_eq!(parse_repeat_input(Some("disable"), true), RepeatAction::Set(false));
        assert_eq!(parse_repeat_input(Some("false"), true), RepeatAction::Set(false));
        assert_eq!(parse_repeat_input(Some("no"), true), RepeatAction::Set(false));
        assert_eq!(parse_repeat_input(Some("0"), true), RepeatAction::Set(false));
        assert_eq!(parse_repeat_input(Some("toggle"), false), RepeatAction::Set(true));
        assert_eq!(parse_repeat_input(Some("toggle"), true), RepeatAction::Set(false));
        assert_eq!(parse_repeat_input(Some("status"), false), RepeatAction::Status);
        assert_eq!(parse_repeat_input(Some("check"), true), RepeatAction::Status);
        assert_eq!(parse_repeat_input(Some("info"), true), RepeatAction::Status);
    }
}

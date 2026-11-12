use crate::commands::gif::fetch_random_gif;
use crate::commands::help::{get_command_help_embed, get_general_help_embed};
use crate::commands::volume::{parse_step_input, parse_volume_input};
use crate::commands::xkcd::get_random_xkcd;
use crate::myauth::AuthData;

use serenity::async_trait;
use serenity::builder::{
    CreateCommand, CreateCommandOption, EditInteractionResponse,
};
use serenity::client::Context;
use serenity::model::application::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, CommandOptionType,
};
use serenity::model::id::GuildId;
use sexual_tiger::{
    add_song_to_playlist, clean_youtube_url, create_playlist, delete_playlist, download_song,
    find_song_by_query, get_playlist_by_name, get_playlist_songs, insert_song, list_playlists,
    list_songs, new_song, record_song_play, remove_song_from_playlist, resolve_song_path,
    search_songs, DatabasePool,
};
use songbird::events::{Event, EventContext, EventHandler, TrackEvent};

struct SlashTrackErrorNotifier;

#[async_trait]
impl EventHandler for SlashTrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                eprintln!(
                    "Slash audio track {:?} encountered error. Position: {:?}, Error: {:?}",
                    handle.uuid(),
                    state.position,
                    state.playing
                );
            }
        }
        None
    }
}

pub fn create_slash_commands() -> Vec<CreateCommand> {
    vec![
        // /play
        CreateCommand::new("play")
            .description("Play a song by name, database ID #, or stream audio from YouTube")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "query",
                    "Song name or database ID # from the library, or a YouTube URL",
                )
                .required(true),
            ),
        // /volume
        CreateCommand::new("volume")
            .description("View or adjust playback volume")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "level",
                    "Volume level (e.g. 80, 120%, 1.5)",
                )
                .required(false),
            ),
        // /vup
        CreateCommand::new("vup")
            .description("Increase playback volume by 10% or custom step")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "step",
                    "Amount to increase (e.g. 10%, 20)",
                )
                .required(false),
            ),
        // /vdown
        CreateCommand::new("vdown")
            .description("Decrease playback volume by 10% or custom step")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "step",
                    "Amount to decrease (e.g. 10%, 20)",
                )
                .required(false),
            ),
        // /skip
        CreateCommand::new("skip").description("Skip the currently playing track"),
        // /stop
        CreateCommand::new("stop").description("Stop audio playback and clear the queue"),
        // /pause
        CreateCommand::new("pause").description("Pause the currently playing track"),
        // /resume
        CreateCommand::new("resume").description("Resume playback of a paused track"),
        // /queue
        CreateCommand::new("queue").description("View how many songs are in the queue"),
        // /songs
        CreateCommand::new("songs")
            .description("Display top 100 played songs table (with IDs) or search by name/ID")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "query",
                    "Search term or database ID # to filter songs",
                )
                .required(false),
            ),
        // /download
        CreateCommand::new("download")
            .description("Download audio from YouTube and save to library")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "url",
                    "YouTube URL to download",
                )
                .required(true),
            ),
        // /playlist
        CreateCommand::new("playlist")
            .description("Manage and play custom playlists")
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "list",
                "View all saved playlists",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "show",
                    "View songs in a playlist",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "name",
                        "Name of the playlist",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "create",
                    "Create a new playlist",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "name",
                        "Name of the new playlist",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "delete",
                    "Delete an existing playlist",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "name",
                        "Name of the playlist to delete",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "add",
                    "Add a song to a playlist",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "playlist",
                        "Name of the playlist",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "song",
                        "Name or query of the song to add",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "remove",
                    "Remove a song from a playlist",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "playlist",
                        "Name of the playlist",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "song",
                        "Name or query of the song to remove",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "play",
                    "Queue and play an entire playlist",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "name",
                        "Name of the playlist to play",
                    )
                    .required(true),
                ),
            ),
        // /join
        CreateCommand::new("join").description("Join your active voice channel"),
        // /leave
        CreateCommand::new("leave").description("Disconnect from the voice channel"),
        // /fuckoff
        CreateCommand::new("fuckoff").description("Stop playback, clear queue, and leave voice channel"),
        // /mute
        CreateCommand::new("mute").description("Mute the bot in the voice channel"),
        // /unmute
        CreateCommand::new("unmute").description("Unmute the bot in the voice channel"),
        // /deafen
        CreateCommand::new("deafen").description("Deafen the bot in the voice channel"),
        // /undeafen
        CreateCommand::new("undeafen").description("Undeafen the bot in the voice channel"),
        // /help
        CreateCommand::new("help")
            .description("Show all available commands and detailed options")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "command",
                    "Specific command name to get help on",
                )
                .required(false),
            ),
        // /ping
        CreateCommand::new("ping").description("Check bot responsiveness and latency"),
        // /gif
        CreateCommand::new("gif")
            .description("Search Giphy and post a random GIF")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "query",
                    "Search tags or keywords",
                )
                .required(true),
            ),
        // /xkcd
        CreateCommand::new("xkcd").description("Display a random XKCD comic"),
    ]
}

fn get_opt_str<'a>(options: &'a [CommandDataOption], name: &str) -> Option<&'a str> {
    for opt in options {
        if opt.name == name {
            if let CommandDataOptionValue::String(s) = &opt.value {
                return Some(s.as_str());
            }
        }
    }
    None
}

pub async fn handle_slash_command(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(g) => g,
        None => {
            let edit = EditInteractionResponse::new().content("Slash commands must be used within a server.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    // Acknowledge immediately to avoid Discord's 3-second timeout
    if let Err(e) = command.defer(&ctx.http).await {
        eprintln!("Failed to defer slash command {}: {:?}", command.data.name, e);
        return;
    }

    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabasePool>().cloned()
    };

    let pool = match pool {
        Some(p) => p,
        None => {
            let edit = EditInteractionResponse::new().content("Internal error: Database pool unavailable.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let name = command.data.name.as_str();

    match name {
        "play" => {
            let query = get_opt_str(&command.data.options, "query").unwrap_or_default().trim();
            if query.is_empty() {
                let edit = EditInteractionResponse::new().content("Must provide a song name or URL.");
                let _ = command.edit_response(&ctx.http, edit).await;
                return;
            }
            handle_play(ctx, command, guild_id, &pool, query).await;
        }
        "volume" => {
            let level = get_opt_str(&command.data.options, "level");
            handle_volume(ctx, command, guild_id, level).await;
        }
        "vup" => {
            let step = get_opt_str(&command.data.options, "step");
            handle_vup(ctx, command, guild_id, step).await;
        }
        "vdown" => {
            let step = get_opt_str(&command.data.options, "step");
            handle_vdown(ctx, command, guild_id, step).await;
        }
        "skip" => {
            handle_skip(ctx, command, guild_id).await;
        }
        "stop" => {
            handle_stop(ctx, command, guild_id).await;
        }
        "pause" => {
            handle_pause(ctx, command, guild_id).await;
        }
        "resume" => {
            handle_resume(ctx, command, guild_id).await;
        }
        "queue" => {
            handle_queue(ctx, command, guild_id).await;
        }
        "songs" => {
            let query = get_opt_str(&command.data.options, "query");
            handle_songs(ctx, command, &pool, query).await;
        }
        "download" => {
            let url = get_opt_str(&command.data.options, "url").unwrap_or_default().trim();
            if url.is_empty() {
                let edit = EditInteractionResponse::new().content("Must provide a valid URL.");
                let _ = command.edit_response(&ctx.http, edit).await;
                return;
            }
            handle_download(ctx, command, &pool, url).await;
        }
        "playlist" => {
            handle_playlist(ctx, command, guild_id, &pool).await;
        }
        "join" => {
            handle_join(ctx, command, guild_id).await;
        }
        "leave" => {
            handle_leave(ctx, command, guild_id).await;
        }
        "fuckoff" => {
            handle_fuckoff(ctx, command, guild_id).await;
        }
        "mute" => {
            handle_mute(ctx, command, guild_id, true).await;
        }
        "unmute" => {
            handle_mute(ctx, command, guild_id, false).await;
        }
        "deafen" => {
            handle_deafen(ctx, command, guild_id, true).await;
        }
        "undeafen" => {
            handle_deafen(ctx, command, guild_id, false).await;
        }
        "help" => {
            let specific = get_opt_str(&command.data.options, "command");
            handle_help(ctx, command, specific).await;
        }
        "ping" => {
            let edit = EditInteractionResponse::new().content("Pong! 🏓");
            let _ = command.edit_response(&ctx.http, edit).await;
        }
        "gif" => {
            let query = get_opt_str(&command.data.options, "query").unwrap_or_default();
            handle_gif(ctx, command, query).await;
        }
        "xkcd" => {
            handle_xkcd(ctx, command).await;
        }
        _ => {
            let edit = EditInteractionResponse::new().content(format!("Unknown command: {}", name));
            let _ = command.edit_response(&ctx.http, edit).await;
        }
    }
}

async fn ensure_voice_connection(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) -> Result<(), String> {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird client initialized")
        .clone();

    if manager.get(guild_id).is_some() {
        return Ok(());
    }

    // Attempt to auto-join the user's voice channel
    let connect_to = ctx
        .cache
        .guild(guild_id)
        .and_then(|g| g.voice_states.get(&command.user.id).and_then(|vs| vs.channel_id));

    match connect_to {
        Some(channel) => {
            let _ = manager.join(guild_id, channel).await;
            Ok(())
        }
        None => Err("You are not in a voice channel. Please join one first!".to_string()),
    }
}

async fn handle_play(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    pool: &sqlx::SqlitePool,
    query: &str,
) {
    if let Err(e) = ensure_voice_connection(ctx, command, guild_id).await {
        let edit = EditInteractionResponse::new().content(e);
        let _ = command.edit_response(&ctx.http, edit).await;
        return;
    }

    let manager = songbird::get(ctx).await.unwrap().clone();
    let handler_lock = match manager.get(guild_id) {
        Some(h) => h,
        None => {
            let edit = EditInteractionResponse::new().content("Failed to connect to voice channel.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    // If it's a URL
    if query.starts_with("http://") || query.starts_with("https://") {
        let clean_url = clean_youtube_url(query);
        let input: songbird::input::Input =
            songbird::input::YoutubeDl::new(reqwest::Client::new(), clean_url).into();

        let mut handler = handler_lock.lock().await;
        let track = handler.enqueue_input(input).await;
        let _ = track.add_event(Event::Track(TrackEvent::Error), SlashTrackErrorNotifier);
        let q_len = handler.queue().len();

        let msg = if q_len > 1 {
            format!("Queued stream (position: **{}**)", q_len)
        } else {
            "Playing stream 🎶".to_string()
        };

        let edit = EditInteractionResponse::new().content(msg);
        let _ = command.edit_response(&ctx.http, edit).await;
        return;
    }

    // Local song lookup
    let maybe_song = match find_song_by_query(pool, query).await {
        Ok(s) => s,
        Err(e) => {
            let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let song = match maybe_song {
        Some(s) => s,
        None => {
            let edit = EditInteractionResponse::new().content(format!(
                "Song not found matching: \"{}\". Use `/songs` to see available songs.",
                query
            ));
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let resolved_source = resolve_song_path(&song.source);
    let input: songbird::input::Input = songbird::input::File::new(resolved_source).into();

    let mut handler = handler_lock.lock().await;
    let track = handler.enqueue_input(input).await;
    let _ = track.add_event(Event::Track(TrackEvent::Error), SlashTrackErrorNotifier);

    if song.is_clip.unwrap_or(0) == 1 {
        let _ = track.set_volume(5.0);
    } else {
        let _ = track.set_volume(1.5);
    }

    let q_len = handler.queue().len();
    let id_info = song
        .song_id
        .map(|id| format!(" (ID: {})", id))
        .unwrap_or_default();
    let msg = if q_len > 1 {
        format!("Queued **{}**{} (position: {})", song.name, id_info, q_len)
    } else {
        format!("Playing **{}**{} 🎶", song.name, id_info)
    };

    if let Some(sid) = song.song_id {
        let plays = song.num_plays.unwrap_or(0);
        let _ = record_song_play(pool, sid, plays).await;
    }

    let edit = EditInteractionResponse::new().content(msg);
    let _ = command.edit_response(&ctx.http, edit).await;
}

async fn handle_volume(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    level: Option<&str>,
) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    let handler_lock = match manager.get(guild_id) {
        Some(h) => h,
        None => {
            let edit = EditInteractionResponse::new().content("Not in a voice channel.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let handler = handler_lock.lock().await;
    let current_track = match handler.queue().current() {
        Some(t) => t,
        None => {
            let edit = EditInteractionResponse::new().content("Nothing is currently playing.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    match level {
        None => {
            let info = current_track.get_info().await;
            let cur_vol = info.as_ref().map(|i| i.volume).unwrap_or(1.0);
            let pct = (cur_vol * 100.0).round() as i32;
            let emoji = if cur_vol == 0.0 { "🔇" } else if cur_vol < 0.8 { "🔉" } else { "🔊" };
            let edit = EditInteractionResponse::new().content(format!(
                "{} Current volume is **{}%** ({:.2}). Use `/volume level:<#>` to change it.",
                emoji, pct, cur_vol
            ));
            let _ = command.edit_response(&ctx.http, edit).await;
        }
        Some(lvl_str) => {
            let new_vol = match parse_volume_input(lvl_str) {
                Some(v) => v,
                None => {
                    let edit = EditInteractionResponse::new().content("Invalid volume level. Examples: `80`, `120%`, `1.5`.");
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
            };

            let _ = current_track.set_volume(new_vol);
            handler.queue().modify_queue(|q| {
                for track in q {
                    let _ = track.set_volume(new_vol);
                }
            });

            let pct = (new_vol * 100.0).round() as i32;
            let emoji = if new_vol == 0.0 { "🔇" } else if new_vol < 0.8 { "🔉" } else { "🔊" };
            let edit = EditInteractionResponse::new().content(format!(
                "{} Volume set to **{}%** ({:.2})",
                emoji, pct, new_vol
            ));
            let _ = command.edit_response(&ctx.http, edit).await;
        }
    }
}

async fn handle_vup(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    step: Option<&str>,
) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    let handler_lock = match manager.get(guild_id) {
        Some(h) => h,
        None => {
            let edit = EditInteractionResponse::new().content("Not in a voice channel.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let handler = handler_lock.lock().await;
    let current_track = match handler.queue().current() {
        Some(t) => t,
        None => {
            let edit = EditInteractionResponse::new().content("Nothing is currently playing.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let delta = parse_step_input(step);
    let info = current_track.get_info().await;
    let cur_vol = info.as_ref().map(|i| i.volume).unwrap_or(1.0);
    let new_vol = (cur_vol + delta).clamp(0.0, 5.0);

    let _ = current_track.set_volume(new_vol);
    handler.queue().modify_queue(|q| {
        for track in q {
            let _ = track.set_volume(new_vol);
        }
    });

    let pct = (new_vol * 100.0).round() as i32;
    let step_pct = (delta * 100.0).round() as i32;
    let edit = EditInteractionResponse::new().content(format!(
        "🔊 Volume increased by **+{}%** to **{}%** ({:.2})",
        step_pct, pct, new_vol
    ));
    let _ = command.edit_response(&ctx.http, edit).await;
}

async fn handle_vdown(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    step: Option<&str>,
) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    let handler_lock = match manager.get(guild_id) {
        Some(h) => h,
        None => {
            let edit = EditInteractionResponse::new().content("Not in a voice channel.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let handler = handler_lock.lock().await;
    let current_track = match handler.queue().current() {
        Some(t) => t,
        None => {
            let edit = EditInteractionResponse::new().content("Nothing is currently playing.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let delta = parse_step_input(step);
    let info = current_track.get_info().await;
    let cur_vol = info.as_ref().map(|i| i.volume).unwrap_or(1.0);
    let new_vol = (cur_vol - delta).clamp(0.0, 5.0);

    let _ = current_track.set_volume(new_vol);
    handler.queue().modify_queue(|q| {
        for track in q {
            let _ = track.set_volume(new_vol);
        }
    });

    let pct = (new_vol * 100.0).round() as i32;
    let step_pct = (delta * 100.0).round() as i32;
    let emoji = if new_vol == 0.0 { "🔇" } else if new_vol < 0.8 { "🔉" } else { "🔊" };
    let edit = EditInteractionResponse::new().content(format!(
        "{} Volume decreased by **-{}%** to **{}%** ({:.2})",
        emoji, step_pct, pct, new_vol
    ));
    let _ = command.edit_response(&ctx.http, edit).await;
}

async fn handle_skip(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        if handler.queue().is_empty() {
            let edit = EditInteractionResponse::new().content("The queue is empty.");
            let _ = command.edit_response(&ctx.http, edit).await;
        } else {
            let _ = handler.queue().skip();
            let edit = EditInteractionResponse::new().content("Skipped song. ⏭️");
            let _ = command.edit_response(&ctx.http, edit).await;
        }
    } else {
        let edit = EditInteractionResponse::new().content("Not in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn handle_stop(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.stop();
        handler.queue().stop();
        let edit = EditInteractionResponse::new().content("Stopped playback and cleared the queue. ⏹️");
        let _ = command.edit_response(&ctx.http, edit).await;
    } else {
        let edit = EditInteractionResponse::new().content("Not in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn handle_pause(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let msg = match handler.queue().pause() {
            Ok(_) => "Playback paused. ⏸️",
            Err(e) => &format!("Failed to pause: {:?}", e),
        };
        let edit = EditInteractionResponse::new().content(msg);
        let _ = command.edit_response(&ctx.http, edit).await;
    } else {
        let edit = EditInteractionResponse::new().content("Not in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn handle_resume(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let msg = match handler.queue().resume() {
            Ok(_) => "Playback resumed. ▶️",
            Err(e) => &format!("Failed to resume: {:?}", e),
        };
        let edit = EditInteractionResponse::new().content(msg);
        let _ = command.edit_response(&ctx.http, edit).await;
    } else {
        let edit = EditInteractionResponse::new().content("Not in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn handle_queue(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let q_len = handler.queue().len();
        let msg = if q_len == 0 {
            "The queue is currently empty.".to_string()
        } else {
            format!("There are **{}** song(s) in the queue.", q_len)
        };
        let edit = EditInteractionResponse::new().content(msg);
        let _ = command.edit_response(&ctx.http, edit).await;
    } else {
        let edit = EditInteractionResponse::new().content("Not in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn edit_or_followup_chunked(
    ctx: &Context,
    command: &CommandInteraction,
    text: &str,
) {
    use serenity::builder::CreateInteractionResponseFollowup;
    let chunks = sexual_tiger::chunk_text(text, 1900);
    if let Some(first) = chunks.first() {
        let edit = EditInteractionResponse::new().content(first);
        let _ = command.edit_response(&ctx.http, edit).await;
    }
    for subsequent in chunks.iter().skip(1) {
        if !subsequent.trim().is_empty() {
            let followup = CreateInteractionResponseFollowup::new().content(subsequent);
            let _ = command.create_followup(&ctx.http, followup).await;
        }
    }
}

async fn handle_songs(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &sqlx::SqlitePool,
    query: Option<&str>,
) {
    let (title_prefix, songs) = match query {
        Some(q) if !q.trim().is_empty() => {
            let res = search_songs(pool, q.trim()).await;
            (format!("Search results for \"{}\"", q.trim()), res)
        }
        _ => {
            let res = list_songs(pool, 100).await;
            ("Top Played Songs".to_string(), res)
        }
    };

    let songs = match songs {
        Ok(s) => s,
        Err(e) => {
            let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    if songs.is_empty() {
        let edit = EditInteractionResponse::new().content(format!("No songs found for **{}**.", title_prefix));
        let _ = command.edit_response(&ctx.http, edit).await;
        return;
    }

    let pages = sexual_tiger::format_songs_pages(&title_prefix, &songs);
    if let Some(first) = pages.first() {
        let edit = EditInteractionResponse::new().content(first);
        let _ = command.edit_response(&ctx.http, edit).await;
    }
    for page in pages.iter().skip(1) {
        use serenity::builder::CreateInteractionResponseFollowup;
        let followup = CreateInteractionResponseFollowup::new().content(page);
        let _ = command.create_followup(&ctx.http, followup).await;
    }
}

async fn handle_download(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &sqlx::SqlitePool,
    url: &str,
) {
    let downloaded = match download_song(url.to_string()).await {
        Ok(d) => d,
        Err(e) => {
            let edit = EditInteractionResponse::new().content(format!("Failed to download song: {}", e));
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let user_name = command.user.name.clone();
    let new_s = match new_song(&downloaded.metadata, user_name, downloaded.source_path) {
        Ok(s) => s,
        Err(e) => {
            let edit = EditInteractionResponse::new().content(format!("Failed to process downloaded song: {}", e));
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    match insert_song(pool, &new_s).await {
        Ok(s) => {
            let edit = EditInteractionResponse::new().content(format!("Successfully added song: **{}** 🎉", s.name));
            let _ = command.edit_response(&ctx.http, edit).await;
        }
        Err(e) => {
            let err_str = e.to_string();
            let msg = if err_str.contains("UNIQUE constraint failed") {
                format!("Song **{}** is already in the library!", new_s.name)
            } else {
                format!("Error saving song to database: {}", e)
            };
            let edit = EditInteractionResponse::new().content(msg);
            let _ = command.edit_response(&ctx.http, edit).await;
        }
    }
}

async fn handle_playlist(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    pool: &sqlx::SqlitePool,
) {
    let sub = match command.data.options.first() {
        Some(opt) => opt,
        None => {
            let edit = EditInteractionResponse::new().content("Use `/playlist list`, `show`, `create`, `delete`, `add`, `remove`, or `play`.");
            let _ = command.edit_response(&ctx.http, edit).await;
            return;
        }
    };

    let sub_name = sub.name.as_str();
    let sub_opts = match &sub.value {
        CommandDataOptionValue::SubCommand(opts) => opts.as_slice(),
        _ => &[],
    };

    match sub_name {
        "list" => match list_playlists(pool).await {
            Ok(pls) if pls.is_empty() => {
                let edit = EditInteractionResponse::new().content("No playlists found. Create one with `/playlist create name:<name>`!");
                let _ = command.edit_response(&ctx.http, edit).await;
            }
            Ok(pls) => {
                let list: Vec<String> = pls
                    .iter()
                    .map(|p| format!("• **{}** ({} songs) — by {}", p.name, p.num_songs, p.created_by))
                    .collect();
                let text = format!("📑 **Playlists:**\n{}", list.join("\n"));
                edit_or_followup_chunked(ctx, command, &text).await;
            }
            Err(e) => {
                let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
                let _ = command.edit_response(&ctx.http, edit).await;
            }
        },
        "show" => {
            let name = get_opt_str(sub_opts, "name").unwrap_or_default();
            match get_playlist_by_name(pool, name).await {
                Ok(Some(pl)) => {
                    let pl_id = pl.playlist_id.unwrap();
                    let songs = get_playlist_songs(pool, pl_id).await.unwrap_or_default();
                    if songs.is_empty() {
                        let edit = EditInteractionResponse::new().content(format!("Playlist **{}** is empty.", pl.name));
                        let _ = command.edit_response(&ctx.http, edit).await;
                    } else {
                        let list: Vec<String> = songs
                            .iter()
                            .enumerate()
                            .map(|(i, s)| {
                                format!(
                                    "{}. **{}** (ID: {})",
                                    i + 1,
                                    s.name,
                                    s.song_id.unwrap_or(0)
                                )
                            })
                            .collect();
                        let text = format!("📑 **{}** ({} songs):\n{}", pl.name, songs.len(), list.join("\n"));
                        edit_or_followup_chunked(ctx, command, &text).await;
                    }
                }
                Ok(None) => {
                    let edit = EditInteractionResponse::new().content(format!("Playlist \"{}\" not found.", name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
            }
        }
        "create" => {
            let name = get_opt_str(sub_opts, "name").unwrap_or_default();
            let author = &command.user.name;
            match create_playlist(pool, name, author).await {
                Ok(pl) => {
                    let edit = EditInteractionResponse::new().content(format!("Created playlist **{}**! 🎉", pl.name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Failed to create playlist: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
            }
        }
        "delete" => {
            let name = get_opt_str(sub_opts, "name").unwrap_or_default();
            match delete_playlist(pool, name).await {
                Ok(true) => {
                    let edit = EditInteractionResponse::new().content(format!("Deleted playlist **{}**.", name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
                Ok(false) => {
                    let edit = EditInteractionResponse::new().content(format!("Playlist \"{}\" not found.", name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Failed to delete playlist: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
            }
        }
        "add" => {
            let pl_name = get_opt_str(sub_opts, "playlist").unwrap_or_default();
            let song_query = get_opt_str(sub_opts, "song").unwrap_or_default();

            let pl = match get_playlist_by_name(pool, pl_name).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    let edit = EditInteractionResponse::new().content(format!("Playlist \"{}\" not found.", pl_name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
            };

            let song = match find_song_by_query(pool, song_query).await {
                Ok(Some(s)) => s,
                Ok(None) => {
                    let edit = EditInteractionResponse::new().content(format!("Song not found matching \"{}\".", song_query));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
            };

            let song_id = song.song_id.unwrap_or(0);
            match add_song_to_playlist(pool, pl.playlist_id.unwrap(), song_id).await {
                Ok(_) => {
                    let edit = EditInteractionResponse::new().content(format!("Added **{}** (ID: {}) to playlist **{}**! 🎵", song.name, song_id, pl.name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Failed to add song to playlist: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
            }
        }
        "remove" => {
            let pl_name = get_opt_str(sub_opts, "playlist").unwrap_or_default();
            let song_query = get_opt_str(sub_opts, "song").unwrap_or_default();

            let pl = match get_playlist_by_name(pool, pl_name).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    let edit = EditInteractionResponse::new().content(format!("Playlist \"{}\" not found.", pl_name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
            };

            let song = match find_song_by_query(pool, song_query).await {
                Ok(Some(s)) => s,
                Ok(None) => {
                    let edit = EditInteractionResponse::new().content(format!("Song not found matching \"{}\".", song_query));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
            };

            let song_id = song.song_id.unwrap_or(0);
            match remove_song_from_playlist(pool, pl.playlist_id.unwrap(), song_id).await {
                Ok(true) => {
                    let edit = EditInteractionResponse::new().content(format!("Removed **{}** (ID: {}) from playlist **{}**.", song.name, song_id, pl.name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
                Ok(false) => {
                    let edit = EditInteractionResponse::new().content(format!("Song **{}** (ID: {}) is not in playlist **{}**.", song.name, song_id, pl.name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Failed to remove song: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                }
            }
        }
        "play" => {
            let pl_name = get_opt_str(sub_opts, "name").unwrap_or_default();

            if let Err(e) = ensure_voice_connection(ctx, command, guild_id).await {
                let edit = EditInteractionResponse::new().content(e);
                let _ = command.edit_response(&ctx.http, edit).await;
                return;
            }

            let pl = match get_playlist_by_name(pool, pl_name).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    let edit = EditInteractionResponse::new().content(format!("Playlist \"{}\" not found.", pl_name));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Database error: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
            };

            let songs = match get_playlist_songs(pool, pl.playlist_id.unwrap()).await {
                Ok(s) => s,
                Err(e) => {
                    let edit = EditInteractionResponse::new().content(format!("Failed to get playlist songs: {}", e));
                    let _ = command.edit_response(&ctx.http, edit).await;
                    return;
                }
            };

            if songs.is_empty() {
                let edit = EditInteractionResponse::new().content(format!("Playlist **{}** is empty.", pl.name));
                let _ = command.edit_response(&ctx.http, edit).await;
                return;
            }

            let manager = songbird::get(ctx).await.unwrap().clone();
            let handler_lock = manager.get(guild_id).unwrap();

            let mut count = 0;
            {
                let mut handler = handler_lock.lock().await;
                for s in &songs {
                    let path = resolve_song_path(&s.source);
                    let input: songbird::input::Input = songbird::input::File::new(path).into();
                    let track = handler.enqueue_input(input).await;
                    let _ = track.add_event(Event::Track(TrackEvent::Error), SlashTrackErrorNotifier);
                    if s.is_clip.unwrap_or(0) == 1 {
                        let _ = track.set_volume(5.0);
                    } else {
                        let _ = track.set_volume(1.5);
                    }
                    if let Some(sid) = s.song_id {
                        let plays = s.num_plays.unwrap_or(0);
                        let _ = record_song_play(pool, sid, plays).await;
                    }
                    count += 1;
                }
            }

            let edit = EditInteractionResponse::new().content(format!(
                "Queued **{}** songs from playlist **{}**! 🎶",
                count, pl.name
            ));
            let _ = command.edit_response(&ctx.http, edit).await;
        }
        _ => {
            let edit = EditInteractionResponse::new().content("Unknown playlist subcommand.");
            let _ = command.edit_response(&ctx.http, edit).await;
        }
    }
}

async fn handle_join(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let connect_to = ctx
        .cache
        .guild(guild_id)
        .and_then(|g| g.voice_states.get(&command.user.id).and_then(|vs| vs.channel_id));

    match connect_to {
        Some(channel) => {
            let manager = songbird::get(ctx).await.unwrap().clone();
            let _ = manager.join(guild_id, channel).await;
            let edit = EditInteractionResponse::new().content("Joined your voice channel! 🔊");
            let _ = command.edit_response(&ctx.http, edit).await;
        }
        None => {
            let edit = EditInteractionResponse::new().content("You are not in a voice channel. Please join one first!");
            let _ = command.edit_response(&ctx.http, edit).await;
        }
    }
}

async fn handle_leave(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if manager.get(guild_id).is_some() {
        let _ = manager.remove(guild_id).await;
        let edit = EditInteractionResponse::new().content("Left the voice channel. 👋");
        let _ = command.edit_response(&ctx.http, edit).await;
    } else {
        let edit = EditInteractionResponse::new().content("Not currently in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn handle_fuckoff(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        {
            let mut handler = handler_lock.lock().await;
            handler.stop();
            handler.queue().stop();
        }
        let _ = manager.remove(guild_id).await;
        let edit = EditInteractionResponse::new().content("Stopped playback and left voice channel. Bye! 👋");
        let _ = command.edit_response(&ctx.http, edit).await;
    } else {
        let edit = EditInteractionResponse::new().content("Not currently in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn handle_mute(ctx: &Context, command: &CommandInteraction, guild_id: GuildId, mute: bool) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        let _ = handler.mute(mute).await;
        let text = if mute { "Muted 🔇" } else { "Unmuted 🔊" };
        let edit = EditInteractionResponse::new().content(text);
        let _ = command.edit_response(&ctx.http, edit).await;
    } else {
        let edit = EditInteractionResponse::new().content("Not in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn handle_deafen(ctx: &Context, command: &CommandInteraction, guild_id: GuildId, deafen: bool) {
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        let _ = handler.deafen(deafen).await;
        let text = if deafen { "Deafened" } else { "Undeafened" };
        let edit = EditInteractionResponse::new().content(text);
        let _ = command.edit_response(&ctx.http, edit).await;
    } else {
        let edit = EditInteractionResponse::new().content("Not in a voice channel.");
        let _ = command.edit_response(&ctx.http, edit).await;
    }
}

async fn handle_help(ctx: &Context, command: &CommandInteraction, specific: Option<&str>) {
    match specific {
        None => {
            let embed = get_general_help_embed();
            let edit = EditInteractionResponse::new().embed(embed);
            let _ = command.edit_response(&ctx.http, edit).await;
        }
        Some(cmd) => match get_command_help_embed(cmd) {
            Ok(embed) => {
                let edit = EditInteractionResponse::new().embed(embed);
                let _ = command.edit_response(&ctx.http, edit).await;
            }
            Err(msg) => {
                let edit = EditInteractionResponse::new().content(msg);
                let _ = command.edit_response(&ctx.http, edit).await;
            }
        },
    }
}

async fn handle_gif(ctx: &Context, command: &CommandInteraction, query: &str) {
    let data = ctx.data.read().await;
    let auth = data.get::<AuthData>().cloned();
    let api_key = auth.as_ref().map(|a| a.giphy.as_str()).unwrap_or("");

    match fetch_random_gif(api_key, query).await {
        Ok(url) => {
            let edit = EditInteractionResponse::new().content(url);
            let _ = command.edit_response(&ctx.http, edit).await;
        }
        Err(e) => {
            let edit = EditInteractionResponse::new().content(format!("Failed to fetch GIF: {}", e));
            let _ = command.edit_response(&ctx.http, edit).await;
        }
    }
}

async fn handle_xkcd(ctx: &Context, command: &CommandInteraction) {
    match get_random_xkcd().await {
        Ok(embed) => {
            let edit = EditInteractionResponse::new().embed(embed);
            let _ = command.edit_response(&ctx.http, edit).await;
        }
        Err(e) => {
            let edit = EditInteractionResponse::new().content(format!("Failed to fetch XKCD: {}", e));
            let _ = command.edit_response(&ctx.http, edit).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_slash_commands() {
        let commands = create_slash_commands();
        assert_eq!(commands.len(), 23);

        // Check specific command names exist
        let names: Vec<String> = commands
            .iter()
            .map(|c| {
                let val = serde_json::to_value(c).unwrap();
                val.get("name").unwrap().as_str().unwrap().to_string()
            })
            .collect();

        assert!(names.contains(&"play".to_string()));
        assert!(names.contains(&"volume".to_string()));
        assert!(names.contains(&"vup".to_string()));
        assert!(names.contains(&"vdown".to_string()));
        assert!(names.contains(&"playlist".to_string()));
        assert!(names.contains(&"download".to_string()));
        assert!(names.contains(&"help".to_string()));
        assert!(names.contains(&"skip".to_string()));
        assert!(names.contains(&"stop".to_string()));
        assert!(names.contains(&"pause".to_string()));
        assert!(names.contains(&"resume".to_string()));
        assert!(names.contains(&"queue".to_string()));
        assert!(names.contains(&"songs".to_string()));
        assert!(names.contains(&"join".to_string()));
        assert!(names.contains(&"leave".to_string()));
        assert!(names.contains(&"fuckoff".to_string()));
        assert!(names.contains(&"mute".to_string()));
        assert!(names.contains(&"unmute".to_string()));
        assert!(names.contains(&"deafen".to_string()));
        assert!(names.contains(&"undeafen".to_string()));
        assert!(names.contains(&"ping".to_string()));
        assert!(names.contains(&"gif".to_string()));
        assert!(names.contains(&"xkcd".to_string()));
    }
}


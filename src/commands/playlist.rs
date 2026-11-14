use serenity::async_trait;
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
use sexual_tiger::{
    add_song_to_playlist, create_playlist, create_song_track, delete_playlist, find_song_by_query,
    get_playlist_by_name, get_playlist_songs, list_playlists, record_song_play,
    remove_song_from_playlist, resolve_song_path, DatabasePool, SongTrackInfo,
};
use songbird::events::{Event, EventContext, EventHandler, TrackEvent};

struct TrackErrorNotifier;

#[async_trait]
impl EventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                eprintln!(
                    "Playlist audio track {:?} encountered error. Position: {:?}, Error: {:?}",
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
#[aliases("pl")]
pub async fn playlist(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let sub = match args.single::<String>() {
        Ok(s) => s.to_lowercase(),
        Err(_) => {
            send_help(ctx, msg).await;
            return Ok(());
        }
    };

    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabasePool>()
            .cloned()
            .expect("Expected DatabasePool in TypeMap")
    };

    match sub.as_str() {
        "list" | "ls" => {
            match list_playlists(&pool).await {
                Ok(playlists) => {
                    if playlists.is_empty() {
                        check_msg(
                            msg.channel_id
                                .say(&ctx.http, "No playlists found. Create one with `~playlist create <name>`!")
                                .await,
                        );
                    } else {
                        let mut cache: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                        let mut pairs = Vec::new();
                        for p in playlists {
                            let creator = if let Some(cached) = cache.get(&p.created_by) {
                                cached.clone()
                            } else {
                                let resolved = crate::commands::resolve_creator_name(&ctx.http, &p.created_by).await;
                                cache.insert(p.created_by.clone(), resolved.clone());
                                resolved
                            };
                            pairs.push((p, creator));
                        }
                        let pages = sexual_tiger::format_playlists_pages(&pairs);
                        for page in pages {
                            crate::commands::send_chunked(ctx, msg.channel_id, &page).await;
                        }
                    }
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Database error listing playlists: {}", e))
                            .await,
                    );
                }
            }
        }
        "create" | "new" => {
            let name = match args.remains() {
                Some(n) if !n.trim().is_empty() => n.trim(),
                _ => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist create <playlist name>`")
                            .await,
                    );
                    return Ok(());
                }
            };

            match create_playlist(&pool, name, &msg.author.name).await {
                Ok(pl) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Created playlist **{}**!", pl.name))
                            .await,
                    );
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Failed to create playlist: {}", e))
                            .await,
                    );
                }
            }
        }
        "delete" | "rm" => {
            let name = match args.remains() {
                Some(n) if !n.trim().is_empty() => n.trim(),
                _ => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist delete <playlist name>`")
                            .await,
                    );
                    return Ok(());
                }
            };

            match delete_playlist(&pool, name).await {
                Ok(true) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Deleted playlist **{}**.", name))
                            .await,
                    );
                }
                Ok(false) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Playlist **{}** not found.", name))
                            .await,
                    );
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Failed to delete playlist: {}", e))
                            .await,
                    );
                }
            }
        }
        "show" | "info" | "view" => {
            let name = match args.remains() {
                Some(n) if !n.trim().is_empty() => n.trim(),
                _ => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist show <playlist name>`")
                            .await,
                    );
                    return Ok(());
                }
            };

            let maybe_pl = match get_playlist_by_name(&pool, name).await {
                Ok(p) => p,
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Database error: {}", e))
                            .await,
                    );
                    return Ok(());
                }
            };

            let pl = match maybe_pl {
                Some(p) => p,
                None => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Playlist **{}** not found.", name))
                            .await,
                    );
                    return Ok(());
                }
            };

            let pl_id = match pl.playlist_id {
                Some(id) => id,
                None => return Ok(()),
            };

            match get_playlist_songs(&pool, pl_id).await {
                Ok(songs) => {
                    if songs.is_empty() {
                        check_msg(
                            msg.channel_id
                                .say(
                                    &ctx.http,
                                    format!("Playlist **{}** is currently empty. Add songs with `~playlist add \"{}\" <song name>`", pl.name, pl.name),
                                )
                                .await,
                        );
                    } else {
                        let creator = crate::commands::resolve_creator_name(&ctx.http, &pl.created_by).await;
                        let mut response = format!(
                            "**Playlist: {}** ({} songs, created by {}):\n",
                            pl.name, pl.num_songs, creator
                        );
                        for (i, s) in songs.iter().enumerate() {
                            let duration_str = match s.duration {
                                Some(d) => format!("{}:{:02}", d / 60, d % 60),
                                None => "unknown".to_string(),
                            };
                            response.push_str(&format!(
                                "{}. **{}** (ID: {}) [{}] ({} plays)\n",
                                i + 1,
                                s.name,
                                s.song_id.unwrap_or(0),
                                duration_str,
                                s.num_plays.unwrap_or(0)
                            ));
                        }
                        crate::commands::send_chunked(ctx, msg.channel_id, &response).await;
                    }
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Failed to get playlist songs: {}", e))
                            .await,
                    );
                }
            }
        }
        "add" => {
            let pl_name = match args.single_quoted::<String>() {
                Ok(n) => n,
                Err(_) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist add \"<playlist name>\" <song query>` (quote the playlist name if it has spaces)")
                            .await,
                    );
                    return Ok(());
                }
            };

            let song_query = match args.remains() {
                Some(q) if !q.trim().is_empty() => q.trim(),
                _ => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist add \"<playlist name>\" <song query>`")
                            .await,
                    );
                    return Ok(());
                }
            };

            let pl = match get_playlist_by_name(&pool, &pl_name).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Playlist **{}** not found.", pl_name))
                            .await,
                    );
                    return Ok(());
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Database error: {}", e))
                            .await,
                    );
                    return Ok(());
                }
            };

            let song = match find_song_by_query(&pool, song_query).await {
                Ok(Some(s)) => s,
                Ok(None) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Song matching **{}** not found.", song_query))
                            .await,
                    );
                    return Ok(());
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Database error: {}", e))
                            .await,
                    );
                    return Ok(());
                }
            };

            let pl_id = pl.playlist_id.unwrap();
            let song_id = song.song_id.unwrap();

            match add_song_to_playlist(&pool, pl_id, song_id).await {
                Ok(()) => {
                    check_msg(
                        msg.channel_id
                            .say(
                                &ctx.http,
                                format!("Added **{}** (ID: {}) to playlist **{}**!", song.name, song_id, pl.name),
                            )
                            .await,
                    );
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Failed to add song to playlist: {}", e))
                            .await,
                    );
                }
            }
        }
        "remove" | "rm-song" => {
            let pl_name = match args.single_quoted::<String>() {
                Ok(n) => n,
                Err(_) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist remove \"<playlist name>\" <song query>` (quote the playlist name if it has spaces)")
                            .await,
                    );
                    return Ok(());
                }
            };

            let song_query = match args.remains() {
                Some(q) if !q.trim().is_empty() => q.trim(),
                _ => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist remove \"<playlist name>\" <song query>`")
                            .await,
                    );
                    return Ok(());
                }
            };

            let pl = match get_playlist_by_name(&pool, &pl_name).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Playlist **{}** not found.", pl_name))
                            .await,
                    );
                    return Ok(());
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Database error: {}", e))
                            .await,
                    );
                    return Ok(());
                }
            };

            let song = match find_song_by_query(&pool, song_query).await {
                Ok(Some(s)) => s,
                Ok(None) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Song matching **{}** not found.", song_query))
                            .await,
                    );
                    return Ok(());
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Database error: {}", e))
                            .await,
                    );
                    return Ok(());
                }
            };

            let pl_id = pl.playlist_id.unwrap();
            let song_id = song.song_id.unwrap();

            match remove_song_from_playlist(&pool, pl_id, song_id).await {
                Ok(true) => {
                    check_msg(
                        msg.channel_id
                            .say(
                                &ctx.http,
                                format!("Removed **{}** (ID: {}) from playlist **{}**.", song.name, song_id, pl.name),
                            )
                            .await,
                    );
                }
                Ok(false) => {
                    check_msg(
                        msg.channel_id
                            .say(
                                &ctx.http,
                                format!("**{}** (ID: {}) was not found in playlist **{}**.", song.name, song_id, pl.name),
                            )
                            .await,
                    );
                }
                Err(e) => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, format!("Failed to remove song from playlist: {}", e))
                            .await,
                    );
                }
            }
        }
        "play" => {
            let name = match args.remains() {
                Some(n) if !n.trim().is_empty() => n.trim(),
                _ => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist play <playlist name | ID>`")
                            .await,
                    );
                    return Ok(());
                }
            };

            play_playlist_internal(ctx, msg, name, &pool, false).await;
        }
        "shuffle" | "shuf" | "random" => {
            let name = match args.remains() {
                Some(n) if !n.trim().is_empty() => n.trim(),
                _ => {
                    check_msg(
                        msg.channel_id
                            .say(&ctx.http, "Usage: `~playlist shuffle <playlist name | ID>`")
                            .await,
                    );
                    return Ok(());
                }
            };

            play_playlist_internal(ctx, msg, name, &pool, true).await;
        }
        _ => {
            send_help(ctx, msg).await;
        }
    }

    Ok(())
}

#[command]
#[aliases("ppl")]
pub async fn playplaylist(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let name = match args.remains() {
        Some(n) if !n.trim().is_empty() => n.trim(),
        _ => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Usage: `~playplaylist <playlist name | ID>`")
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

    play_playlist_internal(ctx, msg, name, &pool, false).await;
    Ok(())
}

#[command]
#[aliases("spl", "playlistshuffle")]
pub async fn shuffleplaylist(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let name = match args.remains() {
        Some(n) if !n.trim().is_empty() => n.trim(),
        _ => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Usage: `~shuffleplaylist <playlist name | ID>`")
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

    play_playlist_internal(ctx, msg, name, &pool, true).await;
    Ok(())
}

#[command]
#[aliases("shuf")]
pub async fn shuffle(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabasePool>()
            .cloned()
            .expect("Expected DatabasePool in TypeMap")
    };

    if let Some(name) = args.remains().map(|n| n.trim()).filter(|n| !n.is_empty()) {
        play_playlist_internal(ctx, msg, name, &pool, true).await;
        return Ok(());
    }

    // No playlist argument provided: shuffle the existing voice queue
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
        if q_len <= 1 {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Not enough songs in the queue to shuffle.")
                    .await,
            );
            return Ok(());
        }

        queue.modify_queue(|q| {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            let slice = q.make_contiguous();
            if slice.len() > 1 {
                // Keep the active song playing at index 0, shuffle upcoming tracks
                slice[1..].shuffle(&mut rng);
            }
        });

        check_msg(
            msg.channel_id
                .say(&ctx.http, format!("Shuffled **{}** upcoming songs in the queue! 🔀", q_len - 1))
                .await,
        );
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel. Use `~playlist shuffle <name | ID>` or `~play <song>`.")
                .await,
        );
    }

    Ok(())
}

async fn play_playlist_internal(
    ctx: &Context,
    msg: &Message,
    name: &str,
    pool: &sqlx::SqlitePool,
    shuffle: bool,
) {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return;
        }
    };

    let pl = match get_playlist_by_name(pool, name).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Playlist **{}** not found.", name))
                    .await,
            );
            return;
        }
        Err(e) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Database error: {}", e))
                    .await,
            );
            return;
        }
    };

    let pl_id = pl.playlist_id.unwrap();
    let mut songs = match get_playlist_songs(pool, pl_id).await {
        Ok(s) => s,
        Err(e) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed to get playlist songs: {}", e))
                    .await,
            );
            return;
        }
    };

    if songs.is_empty() {
        check_msg(
            msg.channel_id
                .say(&ctx.http, format!("Playlist **{}** is empty.", pl.name))
                .await,
        );
        return;
    }

    if shuffle {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        songs.shuffle(&mut rng);
    }

    let handler_lock = match crate::commands::ensure_voice_channel(ctx, guild_id, msg.author.id).await {
        Ok(h) => h,
        Err(e) => {
            check_msg(msg.channel_id.say(&ctx.http, e).await);
            return;
        }
    };

    let mut count = 0;
    {
        let mut handler = handler_lock.lock().await;
        for s in &songs {
            let path = resolve_song_path(&s.source);
            let input: songbird::input::Input = songbird::input::File::new(path).into();
            let track_info = SongTrackInfo {
                title: s.name.clone(),
                duration_secs: s.duration.map(|d| d as u64),
                song_id: s.song_id,
            };
            let track = handler.enqueue(create_song_track(input, track_info)).await;
            let _ = track.add_event(Event::Track(TrackEvent::Error), TrackErrorNotifier);
            if s.is_clip.unwrap_or(0) == 1 {
                let _ = track.set_volume(sexual_tiger::CLIP_VOLUME);
            } else {
                let _ = track.set_volume(sexual_tiger::DEFAULT_BOT_VOLUME);
            }
            if let Some(sid) = s.song_id {
                let plays = s.num_plays.unwrap_or(0);
                let _ = record_song_play(pool, sid, plays).await;
            }
            count += 1;
        }
    }

    let shuffle_tag = if shuffle { " (shuffled 🔀)" } else { "" };
    check_msg(
        msg.channel_id
            .say(
                &ctx.http,
                format!("Queued **{}** songs from playlist **{}**{}!", count, pl.name, shuffle_tag),
            )
            .await,
    );
}

async fn send_help(ctx: &Context, msg: &Message) {
    let help_text = "**Playlist Commands:**\n\
        • `~playlist list` — View all playlists in a table with IDs\n\
        • `~playlist show <name | ID#>` — View songs in a playlist\n\
        • `~playlist create <name>` — Create a new playlist\n\
        • `~playlist delete <name | ID#>` — Delete a playlist\n\
        • `~playlist add \"<playlist>\" <song>` — Add a song to a playlist\n\
        • `~playlist remove \"<playlist>\" <song>` — Remove a song from a playlist\n\
        • `~playlist play <name | ID#>` (or `~playplaylist <name | ID#>`) — Queue all songs in order\n\
        • `~playlist shuffle <name | ID#>` (or `~shuffleplaylist <name | ID#>`) — Queue all songs shuffled 🔀\n";
    check_msg(msg.channel_id.say(&ctx.http, help_text).await);
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

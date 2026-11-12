use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
use sexual_tiger::{
    add_song_to_playlist, create_playlist, delete_playlist, find_song_by_query,
    get_playlist_by_name, get_playlist_songs, list_playlists, record_song_play,
    remove_song_from_playlist, DatabasePool,
};

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
                        let mut response = String::from("**Playlists:**\n");
                        for p in playlists {
                            response.push_str(&format!(
                                "• **{}** ({} songs) — created by {}\n",
                                p.name, p.num_songs, p.created_by
                            ));
                        }
                        check_msg(msg.channel_id.say(&ctx.http, response).await);
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
                        let mut response = format!(
                            "**Playlist: {}** ({} songs, created by {}):\n",
                            pl.name, pl.num_songs, pl.created_by
                        );
                        for (i, s) in songs.iter().enumerate() {
                            let duration_str = match s.duration {
                                Some(d) => format!("{}:{:02}", d / 60, d % 60),
                                None => "unknown".to_string(),
                            };
                            response.push_str(&format!(
                                "{}. **{}** [{}] ({} plays)\n",
                                i + 1,
                                s.name,
                                duration_str,
                                s.num_plays.unwrap_or(0)
                            ));
                        }
                        check_msg(msg.channel_id.say(&ctx.http, response).await);
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
                                format!("Added **{}** to playlist **{}**!", song.name, pl.name),
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
                                format!("Removed **{}** from playlist **{}**.", song.name, pl.name),
                            )
                            .await,
                    );
                }
                Ok(false) => {
                    check_msg(
                        msg.channel_id
                            .say(
                                &ctx.http,
                                format!("**{}** was not found in playlist **{}**.", song.name, pl.name),
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
                            .say(&ctx.http, "Usage: `~playlist play <playlist name>`")
                            .await,
                    );
                    return Ok(());
                }
            };

            play_playlist_internal(ctx, msg, name, &pool).await;
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
                    .say(&ctx.http, "Usage: `~playplaylist <playlist name>`")
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

    play_playlist_internal(ctx, msg, name, &pool).await;
    Ok(())
}

async fn play_playlist_internal(
    ctx: &Context,
    msg: &Message,
    name: &str,
    pool: &sqlx::SqlitePool,
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
    let songs = match get_playlist_songs(pool, pl_id).await {
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

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(h) => h,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Not in a voice channel. Use `~join` first.")
                    .await,
            );
            return;
        }
    };

    let mut count = 0;
    {
        let mut handler = handler_lock.lock().await;
        for s in &songs {
            let input: songbird::input::Input = songbird::input::File::new(s.source.clone()).into();
            let track = handler.enqueue_input(input).await;
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

    check_msg(
        msg.channel_id
            .say(
                &ctx.http,
                format!("Queued **{}** songs from playlist **{}**!", count, pl.name),
            )
            .await,
    );
}

async fn send_help(ctx: &Context, msg: &Message) {
    let help_text = "**Playlist Commands:**\n\
        • `~playlist list` — View all playlists\n\
        • `~playlist show <name>` — View songs in a playlist\n\
        • `~playlist create <name>` — Create a new playlist\n\
        • `~playlist delete <name>` — Delete a playlist\n\
        • `~playlist add \"<playlist>\" <song>` — Add a song to a playlist\n\
        • `~playlist remove \"<playlist>\" <song>` — Remove a song from a playlist\n\
        • `~playlist play <name>` (or `~playplaylist <name>`) — Queue and play all songs in a playlist\n";
    check_msg(msg.channel_id.say(&ctx.http, help_text).await);
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

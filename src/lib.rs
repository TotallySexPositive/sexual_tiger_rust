pub mod models;

use crate::models::{NewSong, Playlist, Song};
use dotenvy::dotenv;
use serenity::prelude::TypeMapKey;
use songbird::input::metadata::YoutubeDlOutput;
use songbird::input::AuxMetadata;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::{
    env,
    fs,
    process::{Command, Stdio},
    str::FromStr,
};
use tokio::process::Command as TokioCommand;

pub struct DatabasePool;

impl TypeMapKey for DatabasePool {
    type Value = SqlitePool;
}

pub async fn establish_connection_pool() -> SqlitePool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "playlists.sql".to_string());
    let connection_string = if database_url.starts_with("sqlite:") {
        database_url
    } else {
        format!("sqlite:{}", database_url)
    };

    let options = SqliteConnectOptions::from_str(&connection_string)
        .unwrap_or_else(|e| panic!("Error parsing {}: {}", connection_string, e))
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .unwrap_or_else(|e| panic!("Error connecting to {}: {}", connection_string, e))
}

pub fn get_song_directory() -> String {
    env::var("SONG_DIRECTORY").unwrap_or_else(|_| {
        if std::path::Path::new("audio").is_dir() && !std::path::Path::new("songs").is_dir() {
            "audio".to_string()
        } else {
            "songs".to_string()
        }
    })
}

pub fn resolve_song_path(source: &str) -> String {
    if std::path::Path::new(source).exists() {
        return source.to_string();
    }
    if source.starts_with("songs/") {
        let alt = source.replacen("songs/", "audio/", 1);
        if std::path::Path::new(&alt).exists() {
            return alt;
        }
    } else if source.starts_with("audio/") {
        let alt = source.replacen("audio/", "songs/", 1);
        if std::path::Path::new(&alt).exists() {
            return alt;
        }
    }
    source.to_string()
}

pub async fn download_song(uri: String) -> Result<AuxMetadata, String> {
    let song_dir = get_song_directory();
    let _dir = fs::create_dir_all(&song_dir).map_err(|e| format!("Couldn't create SONG_DIRECTORY: {}", e))?;

    let ytdl_args = [
        "--print-json",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "infinite",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        &uri,
        "-o",
        &format!("{}/%(title)s.%(ext)s", &song_dir),
    ];

    match Command::new("yt-dlp")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(_) => _ytdl_metadata(&uri).await,
        Err(x) => Err(format!("Something went wrong with yt-dlp: {}", x)),
    }
}

async fn _ytdl_metadata(uri: &str) -> Result<AuxMetadata, String> {
    let ytdl_args = [
        "-j",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "infinite",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        uri,
        "-o",
        "-",
    ];

    let youtube_dl_output = TokioCommand::new("yt-dlp")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|x| format!("Couldn't get the metadata: {}", x))?;

    let o_vec = youtube_dl_output.stderr;

    let end = (&o_vec)
        .iter()
        .position(|el| *el == 0xA)
        .unwrap_or_else(|| o_vec.len());

    let output: YoutubeDlOutput = serde_json::from_slice(&o_vec[..end])
        .map_err(|e| format!("Couldn't parse metadata json: {}", e))?;

    Ok(output.as_aux_metadata())
}

pub fn new_song(meta: &AuxMetadata, added_by: String, source: String) -> NewSong {
    let duration: i32 = match meta.duration {
        Some(d) => d.as_secs().try_into().unwrap_or(0),
        None => 0,
    };
    let is_clip = if duration < 10 { 1 } else { 0 };

    NewSong {
        name: meta.title.clone().unwrap_or_else(|| "Unknown".to_string()),
        hash_id: sha256_file_to_string(&source),
        duration,
        is_clip,
        num_plays: 0,
        last_played: 0,
        url: meta.source_url.clone(),
        source,
        added_by: Some(added_by),
    }
}

pub async fn insert_song(pool: &SqlitePool, song: &NewSong) -> Result<Song, sqlx::Error> {
    sqlx::query_as::<_, Song>(
        r#"
        INSERT INTO song (name, hash_id, duration, is_clip, num_plays, last_played, url, source, added_by)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING song_id, name, hash_id, duration, is_clip, num_plays, last_played, url, source, added_by
        "#,
    )
    .bind(&song.name)
    .bind(&song.hash_id)
    .bind(song.duration)
    .bind(song.is_clip)
    .bind(song.num_plays)
    .bind(song.last_played)
    .bind(&song.url)
    .bind(&song.source)
    .bind(&song.added_by)
    .fetch_one(pool)
    .await
}

pub async fn find_song_by_name_prefix(pool: &SqlitePool, prefix: &str) -> Result<Option<Song>, sqlx::Error> {
    find_song_by_query(pool, prefix).await
}

pub async fn find_song_by_query(pool: &SqlitePool, query: &str) -> Result<Option<Song>, sqlx::Error> {
    // 1. Exact match
    if let Some(song) = sqlx::query_as::<_, Song>("SELECT * FROM song WHERE name = ? LIMIT 1")
        .bind(query)
        .fetch_optional(pool)
        .await?
    {
        return Ok(Some(song));
    }

    // 2. Prefix match
    let prefix = format!("{}%", query);
    if let Some(song) = sqlx::query_as::<_, Song>("SELECT * FROM song WHERE name LIKE ? LIMIT 1")
        .bind(&prefix)
        .fetch_optional(pool)
        .await?
    {
        return Ok(Some(song));
    }

    // 3. Substring match
    let substring = format!("%{}%", query);
    sqlx::query_as::<_, Song>("SELECT * FROM song WHERE name LIKE ? LIMIT 1")
        .bind(&substring)
        .fetch_optional(pool)
        .await
}

pub async fn list_songs(pool: &SqlitePool, limit: i64) -> Result<Vec<Song>, sqlx::Error> {
    sqlx::query_as::<_, Song>("SELECT * FROM song ORDER BY name ASC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn search_songs(pool: &SqlitePool, query: &str) -> Result<Vec<Song>, sqlx::Error> {
    let search = format!("%{}%", query);
    sqlx::query_as::<_, Song>("SELECT * FROM song WHERE name LIKE ? ORDER BY name ASC LIMIT 25")
        .bind(search)
        .fetch_all(pool)
        .await
}

pub async fn record_song_play(pool: &SqlitePool, song_id: i64, current_plays: i32) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE song SET last_played = ?, num_plays = ? WHERE song_id = ?")
        .bind(now)
        .bind(current_plays + 1)
        .bind(song_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_playlist_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Playlist>, sqlx::Error> {
    sqlx::query_as::<_, Playlist>("SELECT * FROM playlist WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
}

pub async fn create_playlist(
    pool: &SqlitePool,
    name: &str,
    created_by: &str,
) -> Result<Playlist, sqlx::Error> {
    sqlx::query_as::<_, Playlist>(
        "INSERT INTO playlist (name, created_by) VALUES (?, ?) RETURNING playlist_id, name, num_songs, created_by",
    )
    .bind(name)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn delete_playlist(pool: &SqlitePool, name: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM playlist WHERE name = ?")
        .bind(name)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_playlists(pool: &SqlitePool) -> Result<Vec<Playlist>, sqlx::Error> {
    sqlx::query_as::<_, Playlist>("SELECT * FROM playlist ORDER BY name ASC")
        .fetch_all(pool)
        .await
}

pub async fn add_song_to_playlist(
    pool: &SqlitePool,
    playlist_id: i64,
    song_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO playlist_song (playlist_id, song_id) VALUES (?, ?)")
        .bind(playlist_id)
        .bind(song_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove_song_from_playlist(
    pool: &SqlitePool,
    playlist_id: i64,
    song_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM playlist_song WHERE relation_id IN (SELECT relation_id FROM playlist_song WHERE playlist_id = ? AND song_id = ? LIMIT 1)",
    )
    .bind(playlist_id)
    .bind(song_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_playlist_songs(pool: &SqlitePool, playlist_id: i64) -> Result<Vec<Song>, sqlx::Error> {
    sqlx::query_as::<_, Song>(
        r#"
        SELECT s.* FROM song s
        INNER JOIN playlist_song ps ON ps.song_id = s.song_id
        WHERE ps.playlist_id = ?
        ORDER BY ps.relation_id ASC
        "#,
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await
}

use data_encoding::HEXUPPER;
use ring::digest::{Context, Digest, SHA256};
use std::fs::File;
use std::io::{BufReader, Read};

fn sha256_digest<R: Read>(mut reader: R) -> Result<Digest, String> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(context.finish())
}

pub fn sha256_file_to_string(path: &str) -> String {
    let input = File::open(path).unwrap_or_else(|e| panic!("Could not open file {}: {}", path, e));
    let reader = BufReader::new(input);
    let digest = sha256_digest(reader).expect("Failed to digest file");
    HEXUPPER.encode(digest.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_connection_and_song_lookup() {
        let pool = establish_connection_pool().await;
        let song = find_song_by_name_prefix(&pool, "Wow")
            .await
            .expect("Query failed");
        assert!(song.is_some(), "Expected to find 'Wow Sound Effect'");
        let s = song.unwrap();
        assert_eq!(s.name, "Wow Sound Effect");
        assert_eq!(s.is_clip, Some(1));
    }

    #[tokio::test]
    async fn test_playlist_operations() {
        let pool = establish_connection_pool().await;

        // Clean up test playlist if leftover
        let _ = delete_playlist(&pool, "Test Playlist").await;

        // Create playlist using helper
        let created = create_playlist(&pool, "Test Playlist", "Tester")
            .await
            .expect("Failed to create playlist");
        let pl_id = created.playlist_id.expect("Expected playlist_id");
        assert_eq!(created.num_songs, 0);

        let pl = get_playlist_by_name(&pool, "Test Playlist")
            .await
            .expect("Query failed")
            .expect("Playlist not found");
        assert_eq!(pl.num_songs, 0);

        // Find existing song
        let song = find_song_by_query(&pool, "Wow")
            .await
            .expect("Query failed")
            .expect("Song not found");
        let song_id = song.song_id.expect("Expected song_id");

        // Add song to playlist
        add_song_to_playlist(&pool, pl_id, song_id)
            .await
            .expect("Failed to add song to playlist");

        // Verify trigger incremented num_songs
        let pl_updated = get_playlist_by_name(&pool, "Test Playlist")
            .await
            .expect("Query failed")
            .expect("Playlist not found");
        assert_eq!(pl_updated.num_songs, 1);

        // Fetch songs in playlist
        let songs = get_playlist_songs(&pool, pl_id)
            .await
            .expect("Failed to get playlist songs");
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].name, "Wow Sound Effect");

        // Check list_playlists includes it
        let all_pls = list_playlists(&pool).await.expect("Failed to list playlists");
        assert!(all_pls.iter().any(|p| p.name == "Test Playlist"));

        // Remove song from playlist
        let removed = remove_song_from_playlist(&pool, pl_id, song_id)
            .await
            .expect("Failed to remove song");
        assert!(removed);

        // Verify trigger decremented num_songs
        let pl_decremented = get_playlist_by_name(&pool, "Test Playlist")
            .await
            .expect("Query failed")
            .expect("Playlist not found");
        assert_eq!(pl_decremented.num_songs, 0);

        // Delete playlist
        let deleted = delete_playlist(&pool, "Test Playlist")
            .await
            .expect("Failed to delete playlist");
        assert!(deleted);

        let pl_deleted = get_playlist_by_name(&pool, "Test Playlist")
            .await
            .expect("Query failed");
        assert!(pl_deleted.is_none());
    }

    #[tokio::test]
    async fn test_song_search_and_query() {
        let pool = establish_connection_pool().await;

        // Exact match
        let s1 = find_song_by_query(&pool, "Wow Sound Effect")
            .await
            .expect("Exact search failed")
            .expect("Expected song");
        assert_eq!(s1.name, "Wow Sound Effect");

        // Prefix match
        let s2 = find_song_by_query(&pool, "Wow")
            .await
            .expect("Prefix search failed")
            .expect("Expected song");
        assert_eq!(s2.name, "Wow Sound Effect");

        // Substring match
        let s3 = find_song_by_query(&pool, "Sound")
            .await
            .expect("Substring search failed")
            .expect("Expected song");
        assert_eq!(s3.name, "Wow Sound Effect");

        // list_songs
        let songs = list_songs(&pool, 10).await.expect("list_songs failed");
        assert!(songs.len() >= 2);

        // search_songs
        let results = search_songs(&pool, "ALARM").await.expect("search_songs failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "YOUR NEW MORNING ALARM");
    }

    #[tokio::test]
    async fn test_symphonia_webm_probing() {
        use songbird::input::codecs::{get_codec_registry, get_probe};
        use songbird::input::File;

        let input: songbird::input::Input = File::new("audio/Wow Sound Effect.webm").into();
        let live = input.make_live_async().await.expect("Failed to make live");
        if let songbird::input::Input::Live(live_input, _) = live {
            let res = live_input.promote(get_codec_registry(), get_probe());
            match res {
                Ok(_) => println!("PROBE SUCCEEDED!"),
                Err(e) => panic!("PROBE FAILED: {:?}", e),
            }
        }
    }
}

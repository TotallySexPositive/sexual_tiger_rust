pub mod models;

use crate::models::{NewSong, Playlist, Song};
use dotenvy::dotenv;
use serenity::prelude::TypeMapKey;
use songbird::input::metadata::YoutubeDlOutput;
use songbird::input::AuxMetadata;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::{
    env, fs,
    process::Stdio,
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
    let path = std::path::Path::new(source);
    if path.exists() {
        return source.to_string();
    }

    // 1. If source contains "/audio/" or starts with "audio/"
    if let Some(idx) = source.find("audio/") {
        let relative = &source[idx..]; // e.g. "audio/stored/wow.mp3-..."
        if std::path::Path::new(relative).exists() {
            return relative.to_string();
        }
        let songs_alt = format!("songs/{}", &source[idx + 6..]);
        if std::path::Path::new(&songs_alt).exists() {
            return songs_alt;
        }
    }

    // 2. If source contains "/songs/" or starts with "songs/"
    if let Some(idx) = source.find("songs/") {
        let relative = &source[idx..]; // e.g. "songs/stored/wow.mp3-..."
        if std::path::Path::new(relative).exists() {
            return relative.to_string();
        }
        let audio_alt = format!("audio/{}", &source[idx + 6..]);
        if std::path::Path::new(&audio_alt).exists() {
            return audio_alt;
        }
    }

    // 3. If source contains "/stored/" or starts with "stored/"
    if let Some(idx) = source.find("stored/") {
        let relative_stored = &source[idx..]; // e.g. "stored/wow.mp3-..."
        let in_audio = format!("audio/{}", relative_stored);
        if std::path::Path::new(&in_audio).exists() {
            return in_audio;
        }
        let in_songs = format!("songs/{}", relative_stored);
        if std::path::Path::new(&in_songs).exists() {
            return in_songs;
        }
    }

    // 4. Try matching by raw filename in common directories
    if let Some(file_name_os) = path.file_name() {
        let file_name = file_name_os.to_string_lossy();
        let search_dirs = [
            "audio",
            "songs",
            "audio/stored",
            "songs/stored",
            "/usr/src/app/audio",
            "/usr/src/app/songs",
            "/usr/src/app/audio/stored",
            "/usr/src/app/songs/stored",
        ];

        for dir in search_dirs {
            let candidate = format!("{}/{}", dir, file_name);
            if std::path::Path::new(&candidate).exists() {
                return candidate;
            }
        }

        // 5. If filename has a hash suffix (e.g. "wow.mp3-1c463522c432a1efb89b59fa220c4d14")
        // check if base name (without hash) exists (e.g. "wow.mp3" or "wow.webm")
        if let Some((base, _)) = file_name.rsplit_once('-') {
            for dir in search_dirs {
                let candidate = format!("{}/{}", dir, base);
                if std::path::Path::new(&candidate).exists() {
                    return candidate;
                }
            }
        }
    }

    source.to_string()
}

pub fn clean_youtube_url(url: &str) -> String {
    if url.contains("youtube.com/watch") || url.contains("m.youtube.com/watch") {
        if url.contains("list=") {
            let mut video_id = None;
            if let Some(idx) = url.find("?v=") {
                let rest = &url[idx + 3..];
                video_id = Some(rest.split('&').next().unwrap_or(rest));
            } else if let Some(idx) = url.find("&v=") {
                let rest = &url[idx + 3..];
                video_id = Some(rest.split('&').next().unwrap_or(rest));
            }

            if let Some(vid) = video_id {
                if !vid.is_empty() {
                    return format!("https://www.youtube.com/watch?v={}", vid);
                }
            }
        }
    } else if url.contains("youtu.be/") && url.contains('?') {
        let base = url.split('?').next().unwrap_or(url);
        return base.to_string();
    }
    url.to_string()
}

pub fn resolve_existing_song_file(
    candidate: &str,
    song_dir: &str,
    title: Option<&str>,
) -> Result<String, String> {
    // 1. Direct or path-resolved candidate
    let resolved = resolve_song_path(candidate);
    if std::path::Path::new(&resolved).is_file() {
        return Ok(candidate.to_string());
    }

    // 2. Relative to song_dir if candidate had a different path prefix
    if let Some(file_name) = std::path::Path::new(candidate).file_name().and_then(|f| f.to_str()) {
        let in_song_dir = format!("{}/{}", song_dir.trim_end_matches('/'), file_name);
        let resolved_in_song = resolve_song_path(&in_song_dir);
        if std::path::Path::new(&resolved_in_song).is_file() {
            return Ok(in_song_dir);
        }
    }

    // 3. Search song_dir, songs, or audio for matching file names
    let candidate_dirs = [song_dir, "songs", "audio"];
    for dir in candidate_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    if let Some(t) = title {
                        if !t.is_empty()
                            && (file_name.starts_with(t)
                                || file_name.contains(t)
                                || t.contains(file_name.as_ref()))
                        {
                            return Ok(format!("{}/{}", dir.trim_end_matches('/'), file_name));
                        }
                    }
                }
            }
        }
    }

    Err(format!(
        "Could not open file {}: No such file or directory (os error 2)",
        candidate
    ))
}

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub metadata: AuxMetadata,
    pub source_path: String,
}

pub async fn download_song(uri: String) -> Result<DownloadResult, String> {
    let song_dir = get_song_directory();
    fs::create_dir_all(&song_dir)
        .map_err(|e| format!("Couldn't create SONG_DIRECTORY: {}", e))?;
    let target_uri = clean_youtube_url(&uri);

    let output_template = format!("{}/%(title)s.%(ext)s", song_dir.trim_end_matches('/'));

    let ytdl_args = [
        "--no-simulate",
        "--dump-json",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        "-o",
        &output_template,
        &target_uri,
    ];

    let output = TokioCommand::new("yt-dlp")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp failed: {}", err.trim()));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let json_line = stdout_str
        .lines()
        .find(|line| line.starts_with('{'))
        .ok_or_else(|| "yt-dlp did not return metadata JSON".to_string())?;

    let json_val: serde_json::Value = serde_json::from_str(json_line)
        .map_err(|e| format!("Failed to parse metadata JSON: {}", e))?;

    let ytdl_output: YoutubeDlOutput = serde_json::from_value(json_val.clone())
        .map_err(|e| format!("Failed to deserialize YoutubeDlOutput: {}", e))?;

    let candidate_filename = json_val
        .get("_filename")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let title = ytdl_output.title.clone().unwrap_or_else(|| "Unknown".to_string());
            format!("{}/{}.webm", song_dir.trim_end_matches('/'), title)
        });

    let actual_filename = resolve_existing_song_file(
        &candidate_filename,
        &song_dir,
        ytdl_output.title.as_deref(),
    )?;

    Ok(DownloadResult {
        metadata: ytdl_output.as_aux_metadata(),
        source_path: actual_filename,
    })
}

pub fn new_song(meta: &AuxMetadata, added_by: String, source: String) -> Result<NewSong, String> {
    let duration: i32 = match meta.duration {
        Some(d) => d.as_secs().try_into().unwrap_or(0),
        None => 0,
    };
    let is_clip = if duration < 10 { 1 } else { 0 };
    let hash_id = sha256_file_to_string(&source)?;

    Ok(NewSong {
        name: meta.title.clone().unwrap_or_else(|| "Unknown".to_string()),
        hash_id,
        duration,
        is_clip,
        num_plays: 0,
        last_played: 0,
        url: meta.source_url.clone(),
        source,
        added_by: Some(added_by),
    })
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

pub fn sha256_file_to_string(path: &str) -> Result<String, String> {
    let resolved = resolve_song_path(path);
    let input = File::open(&resolved)
        .map_err(|e| format!("Could not open file {}: {}", resolved, e))?;
    let reader = BufReader::new(input);
    let digest = sha256_digest(reader).map_err(|e| format!("Failed to digest file: {}", e))?;
    Ok(HEXUPPER.encode(digest.as_ref()))
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

    #[test]
    fn test_clean_youtube_url() {
        let dirty = "https://www.youtube.com/watch?v=-J7J_IWUhls&list=RD-J7J_IWUhls&start_radio=1";
        assert_eq!(
            clean_youtube_url(dirty),
            "https://www.youtube.com/watch?v=-J7J_IWUhls"
        );

        let list_first = "https://www.youtube.com/watch?list=PL123&v=abc12345&index=2";
        assert_eq!(
            clean_youtube_url(list_first),
            "https://www.youtube.com/watch?v=abc12345"
        );

        let mobile = "https://m.youtube.com/watch?v=abc12345&list=PL123&index=2";
        assert_eq!(
            clean_youtube_url(mobile),
            "https://www.youtube.com/watch?v=abc12345"
        );

        let short = "https://youtu.be/-J7J_IWUhls?list=RD-J7J_IWUhls";
        assert_eq!(
            clean_youtube_url(short),
            "https://youtu.be/-J7J_IWUhls"
        );

        let clean = "https://www.youtube.com/watch?v=-J7J_IWUhls";
        assert_eq!(clean_youtube_url(clean), clean);
    }

    #[test]
    fn test_resolve_existing_song_file() {
        // Direct existing file in audio/
        let res = resolve_existing_song_file("audio/Wow Sound Effect.webm", "audio", Some("Wow Sound Effect"));
        assert!(res.is_ok());

        // File specified as songs/ should resolve to audio/ if audio/ exists
        let res_songs = resolve_existing_song_file("songs/Wow Sound Effect.webm", "audio", Some("Wow Sound Effect"));
        assert!(res_songs.is_ok());

        // Legacy absolute path containing /audio/
        let res_legacy = resolve_song_path("/root/sexual_tiger/audio/Wow Sound Effect.webm");
        assert_eq!(res_legacy, "audio/Wow Sound Effect.webm");

        // Legacy absolute path with hash suffix matching base name
        let res_legacy_hash = resolve_song_path("/root/sexual_tiger/audio/stored/Wow Sound Effect.webm-1c463522c432a1efb89b59fa220c4d14");
        assert_eq!(res_legacy_hash, "audio/Wow Sound Effect.webm");

        // Fallback by title search
        let res_title = resolve_existing_song_file("audio/nonexistent.webm", "audio", Some("Wow Sound Effect"));
        assert!(res_title.is_ok());

        // Truly missing file returns error
        let res_missing = resolve_existing_song_file("audio/missing_test_file_xyz.webm", "audio", Some("DefinitelyNotExisting12345"));
        assert!(res_missing.is_err());
    }

    #[test]
    fn test_sha256_file_to_string() {
        let hash = sha256_file_to_string("audio/Wow Sound Effect.webm")
            .expect("Hashing failed");
        assert_eq!(hash, "376DC6E19339DA3D9904A34302C1FDC0723CA45A664A3F217916719F2D5C9B53");
    }
}

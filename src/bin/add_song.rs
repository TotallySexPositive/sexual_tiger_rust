use sexual_tiger::*;
use std::env;

#[tokio::main]
async fn main() {
    let uri = env::args()
        .nth(1)
        .expect("add_song requires a uri argument");

    let pool = establish_connection_pool().await;
    let song_dir = env::var("SONG_DIRECTORY").expect("SONG_DIRECTORY must be set");

    let meta = download_song(uri)
        .await
        .expect("Couldn't download the song");

    let title = meta.title.clone().unwrap_or_else(|| "Unknown".to_string());
    let source_path = format!("{}/{}.webm", song_dir.trim_end_matches('/'), &title);
    let new_s = new_song(&meta, "Steven".to_string(), source_path);

    let song = insert_song(&pool, &new_s)
        .await
        .expect("Error inserting song into database");

    println!("Successfully inserted song: {:?}", song);
}

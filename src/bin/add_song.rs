use sexual_tiger::*;
use std::env;

#[tokio::main]
async fn main() {
    let uri = env::args()
        .nth(1)
        .expect("add_song requires a uri argument");

    let pool = establish_connection_pool().await;

    let downloaded = download_song(uri)
        .await
        .expect("Couldn't download the song");

    let new_s = new_song(&downloaded.metadata, "Steven".to_string(), downloaded.source_path)
        .expect("Failed to process downloaded song");

    let song = insert_song(&pool, &new_s)
        .await
        .expect("Error inserting song into database");

    println!("Successfully inserted song: {:?}", song);
}

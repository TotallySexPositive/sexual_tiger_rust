use self::models::*;
use diesel::prelude::*;
use sexual_tiger::*;
use std::{
    env,
    fmt::format,
    io::{stdin, Read},
};

// fn main() {
//     use crate::schema::song;
//     let connection = &mut establish_connection();
//     let new_post = NewSong {
//         added_by: &"Steven".to_string(),
//         duration: &1000,
//         hash_id: &"asdfqwerzxcv".to_string(),
//         song_id: &10001,
//         is_clip: &0,
//         num_plays: &0,
//         last_played: &0,
//         url: &"yeah no".to_string(),
//         source: &"fuckoff".to_string(),
//         name: &"Some dumb Song".to_string(),
//     };

//     let s = diesel::insert_into(song::table)
//         .values(&new_post)
//         .returning(Song::as_returning())
//         .get_result(connection)
//         .expect("Error inserting song");
//     println!("Song: {:?}", s)
// }

use tokio::runtime::Runtime;

fn main() {
    use crate::schema::song;

    let uri = env::args()
        .nth(1)
        .expect("add_song requires a uri")
        .parse::<String>()
        .expect("Invalid uri");

    let rt = Runtime::new().unwrap();
    let connection = &mut establish_connection();

    let song_dir = env::var("SONG_DIRECTORY").expect("SONG_DIRECTORY must be set");

    let meta = rt
        .block_on(download_song(uri))
        .expect("Couldn't download the song");

    let s = diesel::insert_into(song::table)
        .values(&new_song(
            meta.clone(),
            "Steven".to_string(),
            format!("{}{}.webm", &song_dir, &meta.title.as_ref().unwrap()),
        ))
        .returning(Song::as_returning())
        .get_result(connection)
        .expect("Error inserting song");
    println!("Song: {:?}", s)
}

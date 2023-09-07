pub mod models;
pub mod schema;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenvy::dotenv;
use models::NewSong;
use serenity::collector::CollectorError;
use songbird::input::Metadata;
use std::{
    env,
    error::Error,
    fs,
    process::{Command, Stdio},
};
use tokio::{process::Command as TokioCommand, task};

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub async fn download_song(uri: String) -> Result<Metadata, String> {
    let song_dir = env::var("SONG_DIRECTORY").expect("SONG_DIRECTORY must be set");
    print!("{}", song_dir);
    let dir: () = fs::create_dir_all(&song_dir).expect("Couldn't create SONG_DIRECTORY");

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

    let rtn = match Command::new("youtube-dl")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(_) => _ytdl_metadata(&uri).await,
        Err(x) => Err(format!("Something went wrong with youtube-dl: {}", x)),
    };

    rtn
}

async fn _ytdl_metadata(uri: &str) -> Result<Metadata, String> {
    // Most of these flags are likely unused, but we want identical search
    // and/or selection as the above functions.
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

    let youtube_dl_output = TokioCommand::new("youtube-dl")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|x| format!("Couldn't get the metadata {}", x))?;

    let o_vec = youtube_dl_output.stderr;

    let end = (&o_vec)
        .iter()
        .position(|el| *el == 0xA)
        .unwrap_or_else(|| o_vec.len());

    let value = serde_json::from_slice(&o_vec[..end]).expect("Couldn't parse the json");

    let metadata = Metadata::from_ytdl_output(value);

    Ok(metadata)
}

pub fn new_song(meta: Metadata, added_by: String, source: String) -> NewSong {
    let duration: i32 = match &meta.duration.unwrap().as_secs().try_into() {
        Ok(x) => *x,
        Err(x) => 0,
    };
    let is_clip = duration < 10;
    NewSong {
        added_by: added_by,
        duration: duration,
        hash_id: sha256_file_to_string(&source),
        is_clip: is_clip.try_into().expect("Should only be 1 or 0"),
        num_plays: 0,
        last_played: 0,
        url: meta.source_url.unwrap(),
        source: source,
        name: meta.title.unwrap(),
    }
}

use data_encoding::HEXUPPER;
use ring::digest::{Context, Digest, SHA256};
use std::fs::File;
use std::io::{BufReader, Read, Write};

fn sha256_digest<R: Read>(mut reader: R) -> Result<Digest, String> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(context.finish())
}

pub fn sha256_file_to_string(path: &String) -> String {
    let input = File::open(path).unwrap();
    let reader = BufReader::new(input);
    let digest = sha256_digest(reader).unwrap();
    HEXUPPER.encode(digest.as_ref())
}

// fn main() -> Result<()> {
//     let path = "file.txt";

//     let mut output = File::create(path)?;
//     write!(output, "We will generate a digest of this text")?;

//     let input = File::open(path)?;
//     let reader = BufReader::new(input);
//     let digest = sha256_digest(reader)?;

//     println!("SHA-256 digest is {}", HEXUPPER.encode(digest.as_ref()));

//     Ok(())
// }

use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Access {
    pub user_id: String,
    pub command: String,
    pub is_allowed: i32,
    pub set_by: Option<String>,
    pub added_at: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Command {
    pub command: String,
    pub default_access: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct Image {
    pub image_id: i64,
    pub hash_id: String,
    pub extension: String,
    pub added_by: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct ImageTag {
    pub image_tag_id: i64,
    pub image_id: i64,
    pub tag_id: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct Member {
    pub member_id: Option<i64>,
    pub username: String,
    pub added_at: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Playlist {
    pub playlist_id: Option<i64>,
    pub name: String,
    pub num_songs: i32,
    pub created_by: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct PlaylistSong {
    pub relation_id: Option<i64>,
    pub playlist_id: i64,
    pub song_id: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct Song {
    pub song_id: Option<i64>,
    pub name: String,
    pub hash_id: String,
    pub duration: Option<i32>,
    pub is_clip: Option<i32>,
    pub num_plays: Option<i32>,
    pub last_played: Option<i64>,
    pub url: Option<String>,
    pub source: String,
    pub added_by: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Tag {
    pub tag_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewSong {
    pub name: String,
    pub hash_id: String,
    pub duration: i32,
    pub is_clip: i32,
    pub num_plays: i32,
    pub last_played: i64,
    pub url: Option<String>,
    pub source: String,
    pub added_by: Option<String>,
}

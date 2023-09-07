use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::access)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Access {
    pub user_id: String,
    pub command: String,
    pub is_allowed: i32,
    pub set_by: Option<String>,
    pub added_at: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::command)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Command {
    pub command_column: String,
    pub default_access: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::image)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Image {
    pub image_id: i32,
    pub hash_id: String,
    pub extension: String,
    pub added_by: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::image_tag)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ImageTag {
    pub image_tag_id: i32,
    pub image_id: i32,
    pub tag_id: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::member)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Member {
    pub member_id: Option<i32>,
    pub username: String,
    pub added_at: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::playlist)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Playlist {
    pub playlist_id: Option<i32>,
    pub name: String,
    pub num_songs: i32,
    pub created_by: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::playlist_song)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PlaylistSong {
    pub relation_id: Option<i32>,
    pub playlist_id: i32,
    pub song_id: i32,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::song)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Song {
    pub song_id: Option<i32>,
    pub name: String,
    pub hash_id: String,
    pub duration: Option<i32>,
    pub is_clip: Option<i32>,
    pub num_plays: Option<i32>,
    pub last_played: Option<i32>,
    pub url: Option<String>,
    pub source: String,
    pub added_by: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tag)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Tag {
    pub tag_id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::song)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewSong {
    // pub song_id: &'a i32, // generated
    pub name: String,
    pub hash_id: String,
    pub duration: i32,
    pub is_clip: i32,
    pub num_plays: i32,
    pub last_played: i32,
    pub url: String,
    pub source: String,
    pub added_by: String,
}

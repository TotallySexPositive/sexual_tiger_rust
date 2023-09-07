// @generated automatically by Diesel CLI.

diesel::table! {
    access (user_id, command) {
        user_id -> Text,
        command -> Text,
        is_allowed -> Integer,
        set_by -> Nullable<Text>,
        added_at -> Nullable<Text>,
    }
}

diesel::table! {
    command (command_column) {
        #[sql_name = "command"]
        command_column -> Text,
        default_access -> Integer,
    }
}

diesel::table! {
    image (image_id) {
        image_id -> Integer,
        hash_id -> Text,
        extension -> Text,
        added_by -> Text,
    }
}

diesel::table! {
    image_tag (image_tag_id) {
        image_tag_id -> Integer,
        image_id -> Integer,
        tag_id -> Integer,
    }
}

diesel::table! {
    member (member_id) {
        member_id -> Nullable<Integer>,
        username -> Text,
        added_at -> Nullable<Text>,
    }
}

diesel::table! {
    playlist (playlist_id) {
        playlist_id -> Nullable<Integer>,
        name -> Text,
        num_songs -> Integer,
        created_by -> Text,
    }
}

diesel::table! {
    playlist_song (relation_id) {
        relation_id -> Nullable<Integer>,
        playlist_id -> Integer,
        song_id -> Integer,
    }
}

diesel::table! {
    song (song_id) {
        song_id -> Nullable<Integer>,
        name -> Text,
        hash_id -> Text,
        duration -> Nullable<Integer>,
        is_clip -> Nullable<Integer>,
        num_plays -> Nullable<Integer>,
        last_played -> Nullable<Integer>,
        url -> Nullable<Text>,
        source -> Text,
        added_by -> Nullable<Text>,
    }
}

diesel::table! {
    tag (tag_id) {
        tag_id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
    }
}

diesel::joinable!(access -> command (command));
diesel::joinable!(image_tag -> image (image_id));
diesel::joinable!(image_tag -> tag (tag_id));
diesel::joinable!(playlist_song -> playlist (playlist_id));
diesel::joinable!(playlist_song -> song (song_id));

diesel::allow_tables_to_appear_in_same_query!(
    access,
    command,
    image,
    image_tag,
    member,
    playlist,
    playlist_song,
    song,
    tag,
);

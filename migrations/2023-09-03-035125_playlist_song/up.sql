CREATE TABLE IF NOT EXISTS playlist_song (
    relation_id INTEGER PRIMARY KEY,
    playlist_id INTEGER NOT NULL,
    song_id INTEGER NOT NULL,
    FOREIGN KEY(playlist_id) REFERENCES playlist(playlist_id) ON DELETE CASCADE,
    FOREIGN KEY(song_id) REFERENCES song(song_id)
);

CREATE TRIGGER IF NOT EXISTS increment_num_songs AFTER INSERT ON playlist_song
BEGIN
    UPDATE playlist SET num_songs = num_songs + 1 WHERE playlist.playlist_id = NEW.playlist_id;
END;

CREATE TRIGGER IF NOT EXISTS decrement_num_songs AFTER DELETE ON playlist_song
BEGIN
    UPDATE playlist SET num_songs = num_songs - 1 WHERE playlist.playlist_id = OLD.playlist_id;
END;

CREATE INDEX IF NOT EXISTS playlist_id_song_id ON playlist_song (
    playlist_id,
    song_id
);
# 🐯 Sexual Tiger (Rust)

A high-performance, asynchronous Discord music and utility bot written in Rust using [Serenity](https://github.com/serenity-rs/serenity) and [Songbird](https://github.com/serenity-rs/songbird).

Maintains 100% backward compatibility with the legacy SQLite `playlists.sql` schema, volume mounts, and credentials.

---

## ⚡ Features

- **Local & Streamed Audio**: Seamlessly plays local library files (`.webm`, `.m4a`, `.opus`, `.mp3`) and live streams YouTube audio directly into voice channels with low latency.
- **Dynamic Volume Adjustment**: Real-time volume controls (`~volume <#>`, `~vup`, `~vdown`) that adjust the active track and queue on the fly.
- **Smart Downloader**: Uses `yt-dlp` to download songs directly into your audio directory, computes SHA-256 fingerprints to prevent duplicates, and registers metadata in SQLite.
- **Full Playlist Management**: Create, view, edit, and queue custom playlists with a clean subcommand interface (`~playlist`).
- **Interactive In-Discord Help**: Rich embed guide (`~help` and `~help <command>`) documenting every command, option, and example directly in Discord.
- **Docker Ready**: Multi-stage Docker container with automated `yt-dlp` updates, audio volume symlinking, and graceful shutdowns.

---

## 🚀 Quick Start with Docker Compose

The easiest way to run Sexual Tiger is using Docker Compose:

```bash
# 1. Build and start the container in detached mode
docker compose up -d --build

# 2. View logs
docker compose logs -f
```

### Volume Mounts (`docker-compose.yml`)

The bot supports the legacy volume bindings:

| Source | Container Destination | Purpose |
|---|---|---|
| `./playlists.sql` | `/usr/src/app/playlists.sql` | Main SQLite database |
| `./playlists.sql-wal` | `/usr/src/app/playlists.sql-wal` | SQLite WAL file (if enabled) |
| `./playlists.sql-shm` | `/usr/src/app/playlists.sql-shm` | SQLite shared memory file (if enabled) |
| `./audio` (or `./songs`) | `/usr/src/app/audio` | Directory holding downloaded audio files |
| `./images` | `/usr/src/app/images` | Image assets directory |
| `./auth.json` | `/usr/src/app/auth.json` | Legacy credential configuration file |
| `./configure.json` | `/usr/src/app/configure.json` | Optional configuration file |

> [!NOTE]
> The entrypoint script automatically symlinks `/usr/src/app/audio` and `/usr/src/app/songs` so songs stored as `songs/...` in the database resolve transparently.

---

## 🔑 Authentication Configuration

Credentials can be supplied through any of the following methods (in order of priority):

1. **Environment Variables / `.env`:**
   ```env
   DISCORD_TOKEN=your_discord_bot_token_here
   GIPHY_TOKEN=your_giphy_api_key_here
   DATABASE_URL=playlists.sql
   SONG_DIRECTORY=songs/
   ```

2. **`auth.json`:**
   ```json
   {
     "token": "your_discord_bot_token_here",
     "giphy": "your_giphy_api_key_here"
   }
   ```
   *(Also accepts `"discord"` as key for the token)*

---

## 📖 Command Reference

Sexual Tiger supports **both** classic prefix commands (`~`) and modern native Discord slash commands (`/`). Both interfaces share the exact same underlying audio player, volume engine, and SQLite database.

### 🎵 Music & Playback

| Prefix Command | Slash Command | Arguments | Description | Example |
|---|---|---|---|---|
| `~play` | `/play` | `query: <name \| ID# \| url>` | Searches database by ID #, exact title, prefix, or substring; or streams live audio from YouTube | `~play 1`<br>`~play wow`<br>`/play query:https://youtu.be/...` |
| `~song` | `/song` | — | Shows the currently playing song and playback position (e.g. `01:30/2:10`) | `~song`<br>`~np`<br>`/song` |
| `~wii` | `/wii` | — | Plays an anxiety-inducing cursed Wii/Mii theme song at random (autojoins voice) | `~wii`<br>`~mii`<br>`/wii` |
| `~download` | `/download` | `url: <url>` | Downloads audio via `yt-dlp`, computes SHA-256 hash, and inserts into library | `~download https://youtu.be/...`<br>`/download url:https://youtu.be/...` |
| `~skip` | `/skip` | — | Skips the currently playing track | `~skip`<br>`/skip` |
| `~stop` | `/stop` | — | Stops audio playback and clears the entire queue | `~stop`<br>`/stop` |
| `~pause` | `/pause` | — | Pauses playback of the active track | `~pause`<br>`/pause` |
| `~resume` | `/resume` | — | Resumes playback of a paused track | `~resume`<br>`/resume` |
| `~queue` | `/queue` | — | Displays the total number of songs queued | `~queue`<br>`/queue` |
| `~repeat` | `/repeat` | `[enabled: <bool \| on \| off>]` | Repeats the current song in the queue forever until turned off, skipped, or stopped | `~repeat`<br>`~repeat on`<br>`~repeat off`<br>`/repeat enabled:True` |
| `~shuffle` | `/shuffle` | `[playlist: <name \| ID#>]` | Shuffles upcoming tracks in voice queue, or shuffles and plays a playlist (autojoins voice) | `~shuffle`<br>`~shuffle 1`<br>`/shuffle`<br>`/shuffle playlist:Gym Mix` |
| `~songs` | `/songs` | `[query: <term \| ID#>]` | Displays top 100 played songs sorted by plays (or search results) in a formatted table with database IDs | `~songs`<br>`~songs 1`<br>`/songs query:alarm` |

---

### 🔊 Volume Controls

Adjusts playback volume in real-time for the active stream and applies across all upcoming songs in the queue. Standard songs start at **25%** (`0.25`), clips start at **75%** (`0.75`), and the maximum volume is **100%** (`1.0`).

| Prefix Command | Slash Command | Arguments | Description | Example |
|---|---|---|---|---|
| `~volume` | `/volume` | `[level: <#>]` | Displays current volume or sets volume (0% to 100%) | `~volume`<br>`/volume level:50`<br>`~volume 80%`<br>`~volume 0.5` |
| `~vup` | `/vup` | `[step: <#>]` | Doubles current volume (default) or increases by custom step | `~vup`<br>`/vup`<br>`~vup 10%`<br>`/vup step:10` |
| `~vdown` | `/vdown` | `[step: <#>]` | Halves current volume (default) or decreases by custom step | `~vdown`<br>`/vdown`<br>`~vdown 10%`<br>`/vdown step:10` |

**Volume Argument Formats:**
- **Percentages:** `~volume 25%`, `~volume 50%`, `~volume 80%`, `/volume level:80%`
- **Multipliers:** `~volume 0.25` (25%), `~volume 0.5` (50%), `~volume 0.75` (75%), `~volume 1.0` (100%)
- **Integers:** `~volume 25` (25%), `~volume 50` (50%), `~volume 80` (80%), `~volume 1` (100%)

---

### 📑 Playlist Management

| Prefix Command | Slash Command | Arguments | Description | Example |
|---|---|---|---|---|
| `~playlist list` | `/playlist list` | — | View all saved playlists in a formatted table with playlist IDs and creators | `~playlist list`<br>`/playlist list` |
| `~playlist show <name \| ID#>` | `/playlist show` | `name: <name \| ID#>` | View songs in a specific playlist (by name or ID #) | `~playlist show 1`<br>`/playlist show name:Party` |
| `~playlist create <name>` | `/playlist create` | `name: <name>` | Create a new empty playlist | `~playlist create Gym Mix`<br>`/playlist create name:Gym Mix` |
| `~playlist delete <name \| ID#>` | `/playlist delete` | `name: <name \| ID#>` | Permanently delete a playlist (by name or ID #) | `~playlist delete 1`<br>`/playlist delete name:Gym Mix` |
| `~playlist add "<pl \| ID#>" <song>` | `/playlist add` | `playlist: <pl \| ID#> song: <name \| ID#>` | Add song from library (by name or ID #) to playlist | `~playlist add 1 1`<br>`/playlist add playlist:Gym Mix song:alarm` |
| `~playlist remove "<pl \| ID#>" <song>` | `/playlist remove` | `playlist: <pl \| ID#> song: <name \| ID#>` | Remove song from playlist (by name or ID #) | `~playlist remove 1 1`<br>`/playlist remove playlist:Gym Mix song:alarm` |
| `~playlist play <name \| ID#>` | `/playlist play` | `name: <name \| ID#>` | Queue and play all songs in a playlist (automatically joins voice) | `~playlist play 1`<br>`/playlist play name:Gym Mix` |
| `~playplaylist <name \| ID#>` | — | `<name \| ID#>` | Shortcut prefix command to play an entire playlist (automatically joins voice) | `~playplaylist 1` |
| `~playlist shuffle <name \| ID#>` | `/playlist shuffle` | `name: <name \| ID#>` | Shuffle and play all songs in a playlist (automatically joins voice) | `~playlist shuffle 1`<br>`/playlist shuffle name:Gym Mix` |
| `~shuffleplaylist <name \| ID#>` | — | `<name \| ID#>` | Shortcut prefix command to shuffle and play a playlist (automatically joins voice) | `~shuffleplaylist 1` |

---

### 🎙️ Voice & Channel Controls

| Prefix Command | Slash Command | Description |
|---|---|---|
| `~join` | `/join` | Connects the bot to your current voice channel |
| `~leave` | `/leave` | Disconnects the bot from the voice channel |
| `~fuckoff` | `/fuckoff` | Immediately stops playback, clears the queue, and disconnects |
| `~mute` / `~unmute` | `/mute` / `/unmute` | Mutes or unmutes the bot in the voice channel |
| `~deafen` / `~undeafen` | `/deafen` / `/undeafen` | Deafens or undeafens the bot in the voice channel |

---

### 🎲 Utilities & Help

| Prefix Command | Slash Command | Arguments | Description | Example |
|---|---|---|---|---|
| `~help` | `/help` | `[command: <cmd>]` | Displays categorized overview embed or details for a command | `~help`<br>`/help command:volume` |
| `~gif` | `/gif` | `query: <terms>` | Posts a random GIF from Giphy matching tags | `~gif tiger`<br>`/gif query:tiger` |
| `~xkcd` | `/xkcd` | — | Displays a random XKCD comic | `~xkcd`<br>`/xkcd` |
| `~ping` | `/ping` | — | Tests bot responsiveness and latency | `~ping`<br>`/ping` |

---

## 🌐 Web API & DataTables Integration (`tiger.wentzel.dev`)

The bot includes a built-in asynchronous HTTP API server configured via `API_PORT=8080` (default port mapped in `docker-compose.yml`) and `API_HOST=0.0.0.0`. It features full CORS support (`Access-Control-Allow-Origin: *`) for web applications like `tiger.wentzel.dev/audio.html`.

### Available Endpoints

- **`GET /api/songs`** (or **`GET /data.json`**): Returns the full library of songs as a JSON array matching the schema expected by jQuery DataTables:
  ```json
  [
    {
      "song_id": 1,
      "name": "YOUR NEW MORNING ALARM",
      "num_plays": 89,
      "is_clip": 0,
      "duration": 58,
      "username": "Steven",
      "added_by": "Steven",
      "url": "https://www.youtube.com/watch?v=enYdAxVcNZA",
      "source": "songs/YOUR NEW MORNING ALARM.webm",
      "last_played": 1794458872
    }
  ]
  ```
- **`OPTIONS /api/songs`** (and `/data.json`): Handles browser CORS preflight requests (HTTP 204 No Content).
- **`GET /health`**: Returns service health status, song count, and endpoint discovery.

### DataTables Configuration (`/audio.html`)

```javascript
$('#songTable').DataTable({
    "ajax": {
        "url": "http://<server-ip-or-domain>:8080/api/songs", // or reverse-proxied /data.json
        "dataSrc": ""
    },
    columns: [
        { data: 'song_id' },
        { data: 'name' },
        { data: 'num_plays' },
        { data: 'is_clip' },
        { data: 'duration' },
        { data: 'username' }
    ]
});
```

In addition to the live HTTP API, the bot automatically exports and maintains an updated static file at `./html/data.json` (mounted to `/var/www/html/data.json` in Docker) on startup and whenever songs are downloaded or played.

---

## 🛠️ Local Development

### Prerequisites

- **Rust 1.74+** (`rustup default stable`)
- **FFmpeg** (`sudo apt install ffmpeg`)
- **yt-dlp** (`curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp && chmod a+rx /usr/local/bin/yt-dlp`)
- **SQLite3 development headers** (`sudo apt install libsqlite3-dev libssl-dev pkg-config`)

### Running Tests

```bash
cargo test
```

### Running Locally

```bash
cargo run --release
```

---

## 📄 License

Internal project. All rights reserved.

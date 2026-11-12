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
| `~play` | `/play` | `query: <song \| url>` | Searches database (exact, prefix, substring) or streams live audio from YouTube | `~play wow`<br>`/play query:https://youtu.be/...` |
| `~download` | `/download` | `url: <url>` | Downloads audio via `yt-dlp`, computes SHA-256 hash, and inserts into library | `~download https://youtu.be/...`<br>`/download url:https://youtu.be/...` |
| `~skip` | `/skip` | — | Skips the currently playing track | `~skip`<br>`/skip` |
| `~stop` | `/stop` | — | Stops audio playback and clears the entire queue | `~stop`<br>`/stop` |
| `~pause` | `/pause` | — | Pauses playback of the active track | `~pause`<br>`/pause` |
| `~resume` | `/resume` | — | Resumes playback of a paused track | `~resume`<br>`/resume` |
| `~queue` | `/queue` | — | Displays the total number of songs queued | `~queue`<br>`/queue` |
| `~songs` | `/songs` | `[query: <term>]` | Lists first 25 songs or searches song titles matching `query` | `~songs alarm`<br>`/songs query:alarm` |

---

### 🔊 Volume Controls

Adjusts playback volume in real-time for the active stream and applies across all upcoming songs in the queue.

| Prefix Command | Slash Command | Arguments | Description | Example |
|---|---|---|---|---|
| `~volume` | `/volume` | `[level: <#>]` | Displays current volume or sets volume (0% to 500%) | `~volume`<br>`/volume level:80`<br>`~volume 120%`<br>`~volume 1.5` |
| `~vup` | `/vup` | `[step: <#>]` | Increases volume by +10% (default) or by custom step | `~vup`<br>`/vup step:20` |
| `~vdown` | `/vdown` | `[step: <#>]` | Decreases volume by -10% (default) or by custom step | `~vdown`<br>`/vdown step:15` |

**Volume Argument Formats:**
- **Percentages:** `~volume 80%`, `/volume level:80%`
- **Multipliers:** `~volume 1.0` (100%), `~volume 1.5` (150%), `~volume 0.5` (50%)
- **Integers:** `~volume 80` (treated as 80%), `~volume 1` (100%), `~volume 2` (200%)

---

### 📑 Playlist Management

| Prefix Command | Slash Command | Arguments | Description | Example |
|---|---|---|---|---|
| `~playlist list` | `/playlist list` | — | View all saved playlists and total song counts | `~playlist list`<br>`/playlist list` |
| `~playlist show <name>` | `/playlist show` | `name: <name>` | View songs in a specific playlist | `~playlist show Party`<br>`/playlist show name:Party` |
| `~playlist create <name>` | `/playlist create` | `name: <name>` | Create a new empty playlist | `~playlist create Gym Mix`<br>`/playlist create name:Gym Mix` |
| `~playlist delete <name>` | `/playlist delete` | `name: <name>` | Permanently delete a playlist | `~playlist delete Gym Mix`<br>`/playlist delete name:Gym Mix` |
| `~playlist add "<pl>" <song>` | `/playlist add` | `playlist: <pl> song: <song>` | Add song from library to playlist | `~playlist add "Gym Mix" alarm`<br>`/playlist add playlist:Gym Mix song:alarm` |
| `~playlist remove "<pl>" <song>` | `/playlist remove` | `playlist: <pl> song: <song>` | Remove song from playlist | `~playlist remove "Gym Mix" alarm`<br>`/playlist remove playlist:Gym Mix song:alarm` |
| `~playlist play <name>` | `/playlist play` | `name: <name>` | Queue and play all songs in a playlist | `~playlist play Gym Mix`<br>`/playlist play name:Gym Mix` |
| `~playplaylist <name>` | — | `<name>` | Shortcut prefix command to play an entire playlist | `~playplaylist Gym Mix` |

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

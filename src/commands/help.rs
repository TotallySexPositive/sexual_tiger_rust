use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::model::colour::Color;
use serenity::Result as SerenityResult;

#[command]
#[aliases("commands", "halp", "h")]
pub async fn help(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.is_empty() {
        let embed = get_general_help_embed();
        let builder = CreateMessage::new().embed(embed);
        check_msg(msg.channel_id.send_message(&ctx.http, builder).await);
    } else {
        let cmd = args.single::<String>().unwrap_or_default();
        match get_command_help_embed(&cmd) {
            Ok(embed) => {
                let builder = CreateMessage::new().embed(embed);
                check_msg(msg.channel_id.send_message(&ctx.http, builder).await);
            }
            Err(err_msg) => {
                check_msg(msg.channel_id.say(&ctx.http, err_msg).await);
            }
        }
    }
    Ok(())
}

pub fn get_general_help_embed() -> CreateEmbed {
    CreateEmbed::new()
        .title("🐯 Sexual Tiger Bot — Command Guide")
        .color(Color::GOLD)
        .description(
            "Welcome! You can run commands using prefix `~<command>` or modern slash commands `/<command>`.\n\
            For detailed syntax, options, and examples, type `~help <command>` or `/help [command]`.\n",
        )
        .field(
            "🎵 Music & Playback",
            "• `play <name | url>` — Play library song or stream YouTube URL\n\
             • `download <url>` — Download audio to local library and database\n\
             • `skip` — Skip to the next song in the queue\n\
             • `stop` — Stop audio playback and clear the queue\n\
             • `pause` / `resume` — Pause or resume the current stream\n\
             • `queue` — View how many songs are in the queue\n\
             • `songs [query]` — List library songs or search by name",
            false,
        )
        .field(
            "🔊 Volume Controls",
            "• `volume [level]` — View or set volume (e.g. `80`, `120%`, `1.5`)\n\
             • `vup [step]` — Increase volume by 10% (or custom amount)\n\
             • `vdown [step]` — Decrease volume by 10% (or custom amount)",
            false,
        )
        .field(
            "📑 Playlist Management",
            "• `playlist list` — View all saved playlists\n\
             • `playlist show <name>` — List songs in a playlist\n\
             • `playlist create <name>` — Create a new playlist\n\
             • `playlist delete <name>` — Delete an existing playlist\n\
             • `playlist add \"<pl>\" <song>` — Add a song to a playlist\n\
             • `playlist remove \"<pl>\" <song>` — Remove a song from a playlist\n\
             • `playlist play <name>` (or `playplaylist <name>`) — Queue an entire playlist",
            false,
        )
        .field(
            "🎙️ Voice Controls",
            "• `join` — Join your active voice channel\n\
             • `leave` — Disconnect from the voice channel\n\
             • `fuckoff` — Stop playback, clear queue, and leave voice channel\n\
             • `mute` / `unmute` — Mute or unmute the bot\n\
             • `deafen` / `undeafen` — Deafen or undeafen the bot",
            false,
        )
        .field(
            "🎲 Utilities & Fun",
            "• `gif <query>` — Post a random GIF from Giphy\n\
             • `xkcd` — Display an interactive random XKCD comic\n\
             • `ping` — Check bot response latency",
            false,
        )
        .footer(CreateEmbedFooter::new(
            "Supports both ~prefix and /slash commands | Type ~help <command> for details",
        ))
}

pub fn get_command_help_embed(cmd: &str) -> Result<CreateEmbed, String> {
    let clean = cmd.trim().trim_start_matches('~').trim_start_matches('/');
    let (title, description, usage, aliases, examples) = match clean {
        "play" | "p" => (
            "Command: play",
            "Plays a song from the local database or streams audio from a web URL. \
             If the bot is not yet in a voice channel, it will automatically join yours.",
            "• `~play <song name>`\n• `~play <youtube url>`\n• `/play query:<song | url>`",
            "`p` (prefix only)",
            "• `~play wow`\n• `~play alarm`\n• `/play query:https://www.youtube.com/watch?v=dQw4w9WgXcQ`",
        ),
        "download" | "dl" => (
            "Command: download",
            "Downloads audio from a URL using yt-dlp, calculates its SHA-256 hash, and saves it into the SQLite database. \
             Downloaded songs are permanently stored and can immediately be played with `~play <name>` or `/play`.",
            "• `~download <url>`\n• `/download url:<url>`",
            "`dl` (prefix only)",
            "• `~download https://www.youtube.com/watch?v=dQw4w9WgXcQ`\n• `/download url:https://www.youtube.com/watch?v=dQw4w9WgXcQ`",
        ),
        "volume" | "vol" | "v" => (
            "Command: volume",
            "Views or adjusts the playback volume for the current track and all tracks in the queue. \
             Supports percentages (e.g. `80%`, `150%`) or raw numbers (e.g. `80`, `1.5`, `0.5`). Maximum volume is 500% (5.0).",
            "• `~volume` (check volume)\n• `~volume <#>` (set volume)\n• `/volume [level:#]`",
            "`vol`, `v`",
            "• `~volume`\n• `~volume 80`\n• `~volume 120%`\n• `/volume level:80`",
        ),
        "vup" | "volup" | "volumeup" => (
            "Command: vup",
            "Increases the stream volume dynamically while audio is playing. Defaults to +10%, but an optional step can be passed.",
            "• `~vup [amount]`\n• `/vup [step:#]`",
            "`volup`, `volumeup`",
            "• `~vup` (increases volume by 10%)\n• `~vup 20` (increases volume by 20%)\n• `/vup step:20`",
        ),
        "vdown" | "voldown" | "volumedown" => (
            "Command: vdown",
            "Decreases the stream volume dynamically while audio is playing. Defaults to -10%, but an optional step can be passed.",
            "• `~vdown [amount]`\n• `/vdown [step:#]`",
            "`voldown`, `volumedown`",
            "• `~vdown` (decreases volume by 10%)\n• `~vdown 20` (decreases volume by 20%)\n• `/vdown step:20`",
        ),
        "playlist" | "pl" | "playplaylist" | "ppl" => (
            "Command: playlist",
            "Complete playlist management system. Create playlists, add songs from your library, view songs, and play entire playlists.",
            "• `~playlist list` — View all saved playlists\n\
             • `~playlist show <name>` — View songs in a playlist\n\
             • `~playlist create <name>` — Create a new playlist\n\
             • `~playlist delete <name>` — Delete a playlist\n\
             • `~playlist add \"<pl>\" <song>` — Add a song to playlist\n\
             • `~playlist remove \"<pl>\" <song>` — Remove a song from playlist\n\
             • `~playlist play <name>` (or `~playplaylist <name>`) — Queue all songs in playlist\n\
             • `/playlist [subcommand]`",
            "`pl`, `playplaylist`, `ppl`",
            "• `~playlist list`\n\
             • `~playlist create Party Hits`\n\
             • `~playlist add \"Party Hits\" wow`\n\
             • `/playlist play name:Party Hits`",
        ),
        "songs" | "list" => (
            "Command: songs",
            "Lists songs stored in the database. If no search query is provided, displays the first 25 songs alphabetically. \
             If a query is given, searches song titles matching the query.",
            "• `~songs [search term]`\n• `/songs [query:<term>]`",
            "`list` (prefix only)",
            "• `~songs`\n• `~songs alarm`\n• `/songs query:wow`",
        ),
        "skip" | "s" => (
            "Command: skip",
            "Skips the currently playing song and advances to the next song in the queue.",
            "• `~skip`\n• `/skip`",
            "`s` (prefix only)",
            "• `~skip`\n• `/skip`",
        ),
        "stop" => (
            "Command: stop",
            "Stops the current audio playback and clears all pending tracks in the queue.",
            "• `~stop`\n• `/stop`",
            "None",
            "• `~stop`\n• `/stop`",
        ),
        "pause" => (
            "Command: pause",
            "Pauses the currently playing track. Use `~resume` or `/resume` to continue.",
            "• `~pause`\n• `/pause`",
            "None",
            "• `~pause`\n• `/pause`",
        ),
        "resume" => (
            "Command: resume",
            "Resumes playback of a paused track.",
            "• `~resume`\n• `/resume`",
            "None",
            "• `~resume`\n• `/resume`",
        ),
        "queue" | "q" => (
            "Command: queue",
            "Displays the number of songs currently pending in the playback queue.",
            "• `~queue`\n• `/queue`",
            "`q` (prefix only)",
            "• `~queue`\n• `/queue`",
        ),
        "join" | "j" => (
            "Command: join",
            "Connects the bot to the voice channel you are currently in.",
            "• `~join`\n• `/join`",
            "`j` (prefix only)",
            "• `~join`\n• `/join`",
        ),
        "leave" => (
            "Command: leave",
            "Disconnects the bot from the current voice channel.",
            "• `~leave`\n• `/leave`",
            "None",
            "• `~leave`\n• `/leave`",
        ),
        "fuckoff" => (
            "Command: fuckoff",
            "Immediately stops playback, clears the entire queue, and disconnects the bot from the voice channel.",
            "• `~fuckoff`\n• `/fuckoff`",
            "None",
            "• `~fuckoff`\n• `/fuckoff`",
        ),
        "mute" | "unmute" => (
            "Command: mute / unmute",
            "Mutes or unmutes the bot's audio output in the voice channel.",
            "• `~mute` / `~unmute`\n• `/mute` / `/unmute`",
            "None",
            "• `~mute`\n• `/unmute`",
        ),
        "deafen" | "undeafen" => (
            "Command: deafen / undeafen",
            "Deafens or undeafens the bot in the voice channel.",
            "• `~deafen` / `~undeafen`\n• `/deafen` / `/undeafen`",
            "None",
            "• `~deafen`\n• `/undeafen`",
        ),
        "gif" => (
            "Command: gif",
            "Searches Giphy and posts a random GIF matching the specified tag or search terms.",
            "• `~gif <search terms>`\n• `/gif query:<terms>`",
            "None",
            "• `~gif tiger`\n• `/gif query:dance party`",
        ),
        "xkcd" => (
            "Command: xkcd",
            "Fetches a random XKCD comic and provides an interactive button to fetch another one.",
            "• `~xkcd`\n• `/xkcd`",
            "None",
            "• `~xkcd`\n• `/xkcd`",
        ),
        "ping" => (
            "Command: ping",
            "Sends a ping message to check the bot's responsiveness.",
            "• `~ping`\n• `/ping`",
            "None",
            "• `~ping`\n• `/ping`",
        ),
        _ => {
            return Err(format!(
                "Unknown command `~{}`. Use `~help` or `/help` to view all available commands.",
                clean
            ));
        }
    };

    let embed = CreateEmbed::new()
        .title(title)
        .color(Color::GOLD)
        .description(description)
        .field("Usage", usage, false)
        .field("Aliases", aliases, true)
        .field("Examples", examples, false)
        .footer(CreateEmbedFooter::new(
            "Use ~help or /help to see all available commands",
        ));

    Ok(embed)
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

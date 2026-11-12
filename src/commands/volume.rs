use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;

/// Parse user volume input into a normalized float multiplier (0.0 to 5.0).
/// Supports formats:
/// - "50%" -> 0.50
/// - "120%" -> 1.20
/// - "0.8" -> 0.80
/// - "1.5" -> 1.50
/// - "80" -> 0.80 (integers > 5 are treated as percentage)
/// - "1" -> 1.00
/// - "2" -> 2.00
pub fn parse_volume_input(raw: &str) -> Option<f32> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(num_str) = s.strip_suffix('%') {
        let val: f32 = num_str.trim().parse().ok()?;
        return Some((val / 100.0).clamp(0.0, 5.0));
    }

    let val: f32 = s.parse().ok()?;
    if val < 0.0 {
        return Some(0.0);
    }
    // If the input was an integer > 5 (e.g. 10..=500), treat as percentage
    if val > 5.0 {
        return Some((val / 100.0).clamp(0.0, 5.0));
    }
    Some(val.clamp(0.0, 5.0))
}

/// Parse a volume delta step (e.g. "10", "10%", "0.1", "20")
pub fn parse_step_input(raw: Option<&str>) -> f32 {
    match raw {
        Some(s) if !s.trim().is_empty() => {
            let trimmed = s.trim();
            if let Some(pct) = trimmed.strip_suffix('%') {
                if let Ok(v) = pct.trim().parse::<f32>() {
                    return (v / 100.0).abs().clamp(0.01, 2.0);
                }
            }
            if let Ok(v) = trimmed.parse::<f32>() {
                if v > 1.0 {
                    return (v / 100.0).abs().clamp(0.01, 2.0);
                } else if v > 0.0 {
                    return v.clamp(0.01, 2.0);
                }
            }
            0.10
        }
        _ => 0.10,
    }
}

#[command]
#[aliases("vol", "v")]
pub async fn volume(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Not in a voice channel. Use `~join` first.")
                    .await,
            );
            return Ok(());
        }
    };

    let handler = handler_lock.lock().await;
    let current_track = match handler.queue().current() {
        Some(track) => track,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Nothing is currently playing.")
                    .await,
            );
            return Ok(());
        }
    };

    // If no argument, show current volume
    if args.is_empty() {
        let info = current_track.get_info().await;
        let current_vol = info.as_ref().map(|i| i.volume).unwrap_or(1.0);
        let pct = (current_vol * 100.0).round() as i32;
        let emoji = if current_vol == 0.0 {
            "🔇"
        } else if current_vol < 0.8 {
            "🔉"
        } else {
            "🔊"
        };
        check_msg(
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("{} Current volume is **{}%** ({:.2}). Use `~volume <#>` to change it.", emoji, pct, current_vol),
                )
                .await,
        );
        return Ok(());
    }

    let input_str = args.single::<String>().unwrap_or_default();
    let new_vol = match parse_volume_input(&input_str) {
        Some(v) => v,
        None => {
            check_msg(
                msg.channel_id
                    .say(
                        &ctx.http,
                        "Invalid volume. Provide a percentage or multiplier (e.g. `~volume 80`, `~volume 120%`, `~volume 1.5`).",
                    )
                    .await,
            );
            return Ok(());
        }
    };

    let _ = current_track.set_volume(new_vol);
    handler.queue().modify_queue(|q| {
        for track in q {
            let _ = track.set_volume(new_vol);
        }
    });

    let pct = (new_vol * 100.0).round() as i32;
    let emoji = if new_vol == 0.0 {
        "🔇"
    } else if new_vol < 0.8 {
        "🔉"
    } else {
        "🔊"
    };

    check_msg(
        msg.channel_id
            .say(
                &ctx.http,
                format!("{} Volume set to **{}%** ({:.2})", emoji, pct, new_vol),
            )
            .await,
    );

    Ok(())
}

#[command]
#[aliases("volup", "volumeup")]
pub async fn vup(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Not in a voice channel. Use `~join` first.")
                    .await,
            );
            return Ok(());
        }
    };

    let handler = handler_lock.lock().await;
    let current_track = match handler.queue().current() {
        Some(track) => track,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Nothing is currently playing.")
                    .await,
            );
            return Ok(());
        }
    };

    let step = parse_step_input(args.remains());
    let info = current_track.get_info().await;
    let current_vol = info.as_ref().map(|i| i.volume).unwrap_or(1.0);
    let new_vol = (current_vol + step).clamp(0.0, 5.0);

    let _ = current_track.set_volume(new_vol);
    handler.queue().modify_queue(|q| {
        for track in q {
            let _ = track.set_volume(new_vol);
        }
    });

    let pct = (new_vol * 100.0).round() as i32;
    let step_pct = (step * 100.0).round() as i32;

    check_msg(
        msg.channel_id
            .say(
                &ctx.http,
                format!("🔊 Volume increased by **+{}%** to **{}%** ({:.2})", step_pct, pct, new_vol),
            )
            .await,
    );

    Ok(())
}

#[command]
#[aliases("voldown", "volumedown")]
pub async fn vdown(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx.http, "Must be in a server").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Not in a voice channel. Use `~join` first.")
                    .await,
            );
            return Ok(());
        }
    };

    let handler = handler_lock.lock().await;
    let current_track = match handler.queue().current() {
        Some(track) => track,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Nothing is currently playing.")
                    .await,
            );
            return Ok(());
        }
    };

    let step = parse_step_input(args.remains());
    let info = current_track.get_info().await;
    let current_vol = info.as_ref().map(|i| i.volume).unwrap_or(1.0);
    let new_vol = (current_vol - step).clamp(0.0, 5.0);

    let _ = current_track.set_volume(new_vol);
    handler.queue().modify_queue(|q| {
        for track in q {
            let _ = track.set_volume(new_vol);
        }
    });

    let pct = (new_vol * 100.0).round() as i32;
    let step_pct = (step * 100.0).round() as i32;
    let emoji = if new_vol == 0.0 {
        "🔇"
    } else if new_vol < 0.8 {
        "🔉"
    } else {
        "🔊"
    };

    check_msg(
        msg.channel_id
            .say(
                &ctx.http,
                format!("{} Volume decreased by **-{}%** to **{}%** ({:.2})", emoji, step_pct, pct, new_vol),
            )
            .await,
    );

    Ok(())
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_volume_input() {
        assert_eq!(parse_volume_input("100%"), Some(1.0));
        assert_eq!(parse_volume_input("50%"), Some(0.5));
        assert_eq!(parse_volume_input("150%"), Some(1.5));
        assert_eq!(parse_volume_input("500%"), Some(5.0));

        assert_eq!(parse_volume_input("1"), Some(1.0));
        assert_eq!(parse_volume_input("2"), Some(2.0));
        assert_eq!(parse_volume_input("0.5"), Some(0.5));
        assert_eq!(parse_volume_input("1.5"), Some(1.5));

        // Integers > 5 are treated as percentages
        assert_eq!(parse_volume_input("50"), Some(0.5));
        assert_eq!(parse_volume_input("80"), Some(0.8));
        assert_eq!(parse_volume_input("100"), Some(1.0));
        assert_eq!(parse_volume_input("200"), Some(2.0));

        // Edge and invalid cases
        assert_eq!(parse_volume_input("-10"), Some(0.0));
        assert_eq!(parse_volume_input("1000%"), Some(5.0));
        assert_eq!(parse_volume_input(""), None);
        assert_eq!(parse_volume_input("abc"), None);
    }

    #[test]
    fn test_parse_step_input() {
        assert_eq!(parse_step_input(None), 0.10);
        assert_eq!(parse_step_input(Some("")), 0.10);
        assert_eq!(parse_step_input(Some("10")), 0.10);
        assert_eq!(parse_step_input(Some("20")), 0.20);
        assert_eq!(parse_step_input(Some("20%")), 0.20);
        assert_eq!(parse_step_input(Some("0.25")), 0.25);
    }
}


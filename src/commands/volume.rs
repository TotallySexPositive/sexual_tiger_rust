use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
#[allow(unused_imports)]
pub use sexual_tiger::{CLIP_VOLUME, DEFAULT_BOT_VOLUME, MAX_VOLUME};

/// Parse user volume input into a normalized float multiplier (0.0 to 1.0).
/// Supports formats:
/// - "50%" -> 0.50
/// - "100%" -> 1.00
/// - "150%" -> 1.00 (clamped to max 1.0)
/// - "0.25" -> 0.25
/// - "0.5" -> 0.50
/// - "0.75" -> 0.75
/// - "80" -> 0.80 (integers > 1 are treated as percentage)
/// - "50" -> 0.50
/// - "25" -> 0.25
/// - "1" -> 1.00
pub fn parse_volume_input(raw: &str) -> Option<f32> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(num_str) = s.strip_suffix('%') {
        let val: f32 = num_str.trim().parse().ok()?;
        return Some((val / 100.0).clamp(0.0, MAX_VOLUME));
    }

    let val: f32 = s.parse().ok()?;
    if val < 0.0 {
        return Some(0.0);
    }
    // If the input was an integer without decimal point > 5 (e.g. 10..=100), treat as percentage
    if !s.contains('.') && val > 5.0 {
        return Some((val / 100.0).clamp(0.0, MAX_VOLUME));
    }
    Some(val.clamp(0.0, MAX_VOLUME))
}

/// Parse a volume delta step (e.g. "10", "10%", "0.1", "20"). Returns None if not specified (defaulting to double/half).
pub fn parse_step_input(raw: Option<&str>) -> Option<f32> {
    match raw {
        Some(s) if !s.trim().is_empty() => {
            let trimmed = s.trim();
            if let Some(pct) = trimmed.strip_suffix('%') {
                if let Ok(v) = pct.trim().parse::<f32>() {
                    return Some((v / 100.0).abs().clamp(0.01, MAX_VOLUME));
                }
            }
            if let Ok(v) = trimmed.parse::<f32>() {
                if v > 1.0 {
                    return Some((v / 100.0).abs().clamp(0.01, MAX_VOLUME));
                } else if v > 0.0 {
                    return Some(v.clamp(0.01, MAX_VOLUME));
                }
            }
            None
        }
        _ => None,
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
        let current_vol = info.as_ref().map(|i| i.volume).unwrap_or(DEFAULT_BOT_VOLUME);
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
                        "Invalid volume. Provide a percentage or multiplier (e.g. `~volume 25`, `~volume 50%`, `~volume 0.75`, `~volume 1.0`).",
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

    let step_opt = parse_step_input(args.remains());
    let info = current_track.get_info().await;
    let current_vol = info.as_ref().map(|i| i.volume).unwrap_or(DEFAULT_BOT_VOLUME);

    let (new_vol, msg_text) = match step_opt {
        Some(step) => {
            let n = (current_vol + step).clamp(0.0, MAX_VOLUME);
            let pct = (n * 100.0).round() as i32;
            let step_pct = (step * 100.0).round() as i32;
            let emoji = if n == 0.0 { "🔇" } else if n < 0.8 { "🔉" } else { "🔊" };
            (n, format!("{} Volume increased by **+{}%** to **{}%** ({:.2})", emoji, step_pct, pct, n))
        }
        None => {
            let n = if current_vol == 0.0 {
                0.05
            } else {
                (current_vol * 2.0).clamp(0.0, MAX_VOLUME)
            };
            let pct = (n * 100.0).round() as i32;
            let emoji = if n == 0.0 { "🔇" } else if n < 0.8 { "🔉" } else { "🔊" };
            let text = if current_vol >= MAX_VOLUME && n >= MAX_VOLUME {
                format!("{} Volume is already at maximum **{}%** ({:.2})", emoji, pct, n)
            } else {
                format!("{} Volume doubled to **{}%** ({:.2})", emoji, pct, n)
            };
            (n, text)
        }
    };

    let _ = current_track.set_volume(new_vol);
    handler.queue().modify_queue(|q| {
        for track in q {
            let _ = track.set_volume(new_vol);
        }
    });

    check_msg(msg.channel_id.say(&ctx.http, msg_text).await);

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

    let step_opt = parse_step_input(args.remains());
    let info = current_track.get_info().await;
    let current_vol = info.as_ref().map(|i| i.volume).unwrap_or(DEFAULT_BOT_VOLUME);

    let (new_vol, msg_text) = match step_opt {
        Some(step) => {
            let n = (current_vol - step).clamp(0.0, MAX_VOLUME);
            let pct = (n * 100.0).round() as i32;
            let step_pct = (step * 100.0).round() as i32;
            let emoji = if n == 0.0 { "🔇" } else if n < 0.8 { "🔉" } else { "🔊" };
            (n, format!("{} Volume decreased by **-{}%** to **{}%** ({:.2})", emoji, step_pct, pct, n))
        }
        None => {
            let n = (current_vol / 2.0).clamp(0.0, MAX_VOLUME);
            let pct = (n * 100.0).round() as i32;
            let emoji = if n == 0.0 { "🔇" } else if n < 0.8 { "🔉" } else { "🔊" };
            (n, format!("{} Volume halved to **{}%** ({:.2})", emoji, pct, n))
        }
    };

    let _ = current_track.set_volume(new_vol);
    handler.queue().modify_queue(|q| {
        for track in q {
            let _ = track.set_volume(new_vol);
        }
    });

    check_msg(msg.channel_id.say(&ctx.http, msg_text).await);

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
        assert_eq!(parse_volume_input("25%"), Some(0.25));
        assert_eq!(parse_volume_input("75%"), Some(0.75));
        assert_eq!(parse_volume_input("150%"), Some(1.0)); // clamped to 1.0

        assert_eq!(parse_volume_input("1"), Some(1.0));
        assert_eq!(parse_volume_input("1.0"), Some(1.0));
        assert_eq!(parse_volume_input("2"), Some(1.0)); // clamped to 1.0
        assert_eq!(parse_volume_input("0.25"), Some(0.25));
        assert_eq!(parse_volume_input("0.5"), Some(0.5));
        assert_eq!(parse_volume_input("0.75"), Some(0.75));

        // Integers > 1 are treated as percentages
        assert_eq!(parse_volume_input("25"), Some(0.25));
        assert_eq!(parse_volume_input("50"), Some(0.5));
        assert_eq!(parse_volume_input("75"), Some(0.75));
        assert_eq!(parse_volume_input("80"), Some(0.8));
        assert_eq!(parse_volume_input("100"), Some(1.0));

        // Edge and invalid cases
        assert_eq!(parse_volume_input("-10"), Some(0.0));
        assert_eq!(parse_volume_input("1000%"), Some(1.0));
        assert_eq!(parse_volume_input(""), None);
        assert_eq!(parse_volume_input("abc"), None);
    }

    #[test]
    fn test_parse_step_input() {
        assert_eq!(parse_step_input(None), None);
        assert_eq!(parse_step_input(Some("")), None);
        assert_eq!(parse_step_input(Some("10")), Some(0.10));
        assert_eq!(parse_step_input(Some("20")), Some(0.20));
        assert_eq!(parse_step_input(Some("20%")), Some(0.20));
        assert_eq!(parse_step_input(Some("0.25")), Some(0.25));
    }

    #[test]
    fn test_volume_constants_and_scaling() {
        assert_eq!(MAX_VOLUME, 1.0);
        assert_eq!(DEFAULT_BOT_VOLUME, 0.25);
        assert_eq!(CLIP_VOLUME, 0.75);

        // Doubling from default 0.25
        let vol_double_1 = (DEFAULT_BOT_VOLUME * 2.0).clamp(0.0, MAX_VOLUME);
        assert_eq!(vol_double_1, 0.50);

        // Doubling from 0.50 to max 1.0
        let vol_double_2 = (vol_double_1 * 2.0).clamp(0.0, MAX_VOLUME);
        assert_eq!(vol_double_2, 1.0);

        // Doubling from 1.0 stays clamped at 1.0
        let vol_double_3 = (vol_double_2 * 2.0).clamp(0.0, MAX_VOLUME);
        assert_eq!(vol_double_3, 1.0);

        // Halving from 1.0
        let vol_half_1 = (vol_double_3 / 2.0).clamp(0.0, MAX_VOLUME);
        assert_eq!(vol_half_1, 0.50);

        // Halving from 0.50
        let vol_half_2 = (vol_half_1 / 2.0).clamp(0.0, MAX_VOLUME);
        assert_eq!(vol_half_2, 0.25);
    }
}


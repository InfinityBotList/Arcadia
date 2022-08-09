use serenity::{http::CacheHttp, model::id::ChannelId};
use sqlx::PgPool;

use crate::types::Error;

pub async fn vote_reset(
    discord: impl CacheHttp,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    crate::staff::add_action_log(pool, &bot_id, staff_id, reason, "vote_reset").await?;

    sqlx::query!("UPDATE bots SET votes = 0 WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    let modlogs = ChannelId(std::env::var("MODLOGS_CHANNEL")?.parse::<u64>()?);

    modlogs
        .send_message(&discord.http(), |m| {
            m.embed(|e| {
                e.title("__Bot Vote Reset!__")
                    .field("Reason", &reason, true)
                    .field("Moderator", "<@".to_string() + &staff_id + ">", true)
                    .field("Bot", "<@".to_string() + &bot_id + ">", true)
                    .footer(|f| f.text("Sad life!"))
                    .color(0xFF0000)
            })
        })
        .await?;

    Ok(())
}

pub async fn vote_reset_all(
    discord: impl CacheHttp,
    pool: &PgPool,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    crate::staff::add_action_log(
        pool,
        &std::env::var("TEST_BOT")?,
        staff_id,
        reason,
        "vote_reset_all",
    )
    .await?;

    sqlx::query!("UPDATE bots SET votes = 0")
        .execute(pool)
        .await?;

    let modlogs = ChannelId(std::env::var("MODLOGS_CHANNEL")?.parse::<u64>()?);

    modlogs
        .send_message(&discord.http(), |m| {
            m.embed(|e| {
                e.title("__All Votes Reset!__")
                    .field("Reason", &reason, true)
                    .field("Moderator", "<@".to_string() + &staff_id + ">", true)
                    .footer(|f| f.text("Sad life!"))
                    .color(0xFF0000)
            })
        })
        .await?;

    Ok(())
}

use std::num::NonZeroU64;

use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter, CreateMessage},
    http::CacheHttp,
    model::id::ChannelId,
};
use sqlx::PgPool;

use crate::types::Error;

pub async fn vote_reset(
    discord: impl CacheHttp,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    crate::staff::add_action_log(pool, bot_id, staff_id, reason, "vote_reset").await?;

    sqlx::query!("UPDATE bots SET votes = 0 WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    let modlogs = ChannelId(std::env::var("MODLOGS_CHANNEL")?.parse::<NonZeroU64>()?);

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__Bot Vote Reset!__")
            .field("Reason", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .field("Bot", "<@".to_string() + bot_id + ">", true)
            .footer(CreateEmbedFooter::new("Sad life :("))
            .color(0xFF0000),
    );

    modlogs.send_message(&discord.http(), msg).await?;

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

    let modlogs = ChannelId(std::env::var("MODLOGS_CHANNEL")?.parse::<NonZeroU64>()?);

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__All Votes Reset!__")
            .field("Reason", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .footer(CreateEmbedFooter::new("Sad life :("))
            .color(0xFF0000),
    );

    modlogs.send_message(&discord.http(), msg).await?;

    Ok(())
}

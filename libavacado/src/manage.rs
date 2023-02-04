use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter, CreateMessage},
    model::id::ChannelId,
};
use sqlx::PgPool;

use crate::{types::Error, types::CacheHttpImpl};

pub async fn vote_reset_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    if bot_id == "all" {
        return Err("You cannot reset all votes with this command".into());
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    crate::staff::add_action_log(pool, bot_id, staff_id, reason, "vote_reset").await?;

    sqlx::query!("UPDATE bots SET votes = 0 WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__Bot Vote Reset!__")
            .field("Reason", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .field("Bot", "<@".to_string() + bot_id + ">", true)
            .footer(CreateEmbedFooter::new("Sad life :("))
            .color(0xFF0000),
    );

    ChannelId(crate::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

pub async fn vote_reset_all_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    // If bot_id is "all", reset all votes
    crate::staff::add_action_log(pool, "all", staff_id, reason, "vote_reset").await?;

    sqlx::query!("UPDATE bots SET votes = 0")
        .execute(pool)
        .await?;

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__All Votes Reset!__")
            .field("Reason", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .footer(CreateEmbedFooter::new("Sad life :("))
            .color(0xFF0000),
    );

    ChannelId(crate::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

pub async fn unverify_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    if bot_id == "all" {
        return Err("You cannot unverify all bots".into());
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    crate::staff::add_action_log(pool, bot_id, staff_id, reason, "unverify").await?;

    sqlx::query!("UPDATE bots SET type = 'pending' WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__Bot Unverified For Futher Review!__")
            .field("Reason", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .field("Bot", "<@".to_string() + bot_id + ">", true)
            .footer(CreateEmbedFooter::new("Gonna be pending further review..."))
            .color(0xFF0000),
    );

    ChannelId(crate::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

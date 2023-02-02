use std::num::NonZeroU64;

/*
Implementation of common staff functions that can be shared between bot and API

Currently main actions are: approve, deny, vote reset (coming soon)

Smaller utilities specific to staff like add_action_log are also here
*/
use crate::types::{ApproveResponse, Error};
use log::info;
use serde::Serialize;
use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter, CreateMessage},
    http::CacheHttp,
};
use sqlx::PgPool;

use serenity::model::id::{ChannelId, UserId};

#[derive(Serialize)]
struct Reason {
    reason: String,
}

/// Records a action log
pub async fn add_action_log(
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
    event_type: &str,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO action_logs (bot_id, staff_id, action_reason, event) VALUES ($1, $2, $3, $4)",
        bot_id,
        staff_id,
        reason,
        event_type
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Approve bot implementation
pub async fn approve_bot(
    discord: impl CacheHttp,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<ApproveResponse, Error> {
    // The bot has way better onboarding, but this is a generic impl function so we need it
    let onboard_state = sqlx::query!(
        "SELECT staff_onboard_state FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    // We should never get this on bot, but maybe on website
    if onboard_state.staff_onboard_state != "completed" {
        return Err("onboarding_required".into());
    }

    if reason.len() < 5 || reason.len() > 1998 {
        return Err("Reason is too short or too long".into());
    }

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(pool)
    .await?;

    let claimed = sqlx::query!(
        "SELECT type, claimed_by, owner, last_claimed FROM bots WHERE bot_id = $1",
        bot_id
    )
    .fetch_one(pool)
    .await?;

    if claimed.r#type != "claimed" {
        return Err("Bot is not pending review?".into());
    }

    if claimed.claimed_by.is_none()
        || claimed.claimed_by.as_ref().unwrap().is_empty()
        || claimed.last_claimed.is_none()
    {
        return Err(format!(
            "<@{}> is not claimed? Do ``/claim`` to claim this bot first!",
            bot_id
        )
        .into());
    }

    let start_time = chrono::offset::Utc::now();
    let last_claimed = claimed.last_claimed.unwrap();

    if (start_time - last_claimed).num_minutes() < 5 {
        return Err("Whoa there! You need to test this bot for at least 5 minutes (recommended: 10-20 minutes) before being able to approve/deny it!".into());
    }

    add_action_log(pool, bot_id, staff_id, reason, "approve").await?;

    sqlx::query!(
        "UPDATE bots SET type = 'approved', claimed_by = NULL WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    // Get main owner and modlogs
    let owner = UserId(claimed.owner.parse::<NonZeroU64>()?);

    let private_channel = owner.create_dm_channel(&discord).await?;

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("Bot Approved!")
            .description(format!("<@{}> has approved <@{}>", staff_id, bot_id))
            .field("Feedback", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .field("Bot", "<@".to_string() + bot_id + ">", true)
            .footer(CreateEmbedFooter::new("Well done, young traveller!"))
            .color(0x00ff00),
    );

    // Clone here is OK, we want to copy the message
    private_channel
        .send_message(&discord.http(), msg.clone())
        .await?;

    ChannelId(crate::CONFIG.channels.mod_logs)
        .send_message(&discord.http(), msg)
        .await?;

    let request = reqwest::Client::new()
        .post(format!(
            "https://catnip.metrobots.xyz/bots/{}/approve",
            bot_id
        ))
        .query(&[("list_id", crate::CONFIG.metro.list_id.clone())])
        .query(&[("reviewer", bot_id)])
        .header("Authorization", crate::CONFIG.metro.secret.clone())
        .json(&Reason {
            reason: reason.to_string(),
        })
        .send()
        .await?;

    if request.status().is_success() {
        info!("Successfully approved bot {} on metro", bot_id);

        let invite_data = sqlx::query!("SELECT invite FROM bots WHERE bot_id = $1", bot_id)
            .fetch_one(pool)
            .await?;

        Ok(ApproveResponse {
            invite: invite_data.invite,
        })
    } else {
        Err("Failed to approve bot on metro (but successful apperov on IBL".into())
    }
}

/// Deny bot implementation
pub async fn deny_bot(
    discord: impl CacheHttp,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    // The bot has way better onboarding, but this is a generic impl function so we need it
    let onboard_state = sqlx::query!(
        "SELECT staff_onboard_state FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    // We should never get this on bot, but maybe on website
    if onboard_state.staff_onboard_state != "completed" {
        return Err("onboarding_required".into());
    }

    if reason.len() < 5 || reason.len() > 1998 {
        return Err("Reason is too short or too long".into());
    }

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(pool)
    .await?;

    let claimed = sqlx::query!(
        "SELECT type, claimed_by, owner, last_claimed FROM bots WHERE bot_id = $1",
        bot_id
    )
    .fetch_one(pool)
    .await?;

    if claimed.r#type != "claimed" {
        return Err("Bot is not pending review?".into());
    }

    if claimed.claimed_by.is_none()
        || claimed.claimed_by.as_ref().unwrap().is_empty()
        || claimed.last_claimed.is_none()
    {
        return Err(format!(
            "<@{}> is not claimed? Do ``/claim`` to claim this bot first!",
            bot_id
        )
        .into());
    }

    let start_time = chrono::offset::Utc::now();
    let last_claimed = claimed.last_claimed.unwrap();

    if (start_time - last_claimed).num_minutes() < 5 {
        return Err("Whoa there! You need to test this bot for at least 5 minutes (recommended: 10-20 minutes) before being able to approve/deny it!".into());
    }

    // Get main owner and modlogs
    let owner = UserId(claimed.owner.parse::<NonZeroU64>()?);

    // Add action logs
    add_action_log(pool, bot_id, staff_id, reason, "deny").await?;

    sqlx::query!(
        "UPDATE bots SET type = 'denied', claimed_by = NULL WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    let private_channel = owner.create_dm_channel(&discord).await?;

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Bot Denied!")
            .description(format!("<@{}> has denied <@{}>", staff_id, bot_id))
            .field("Reason", reason, true)
            .footer(CreateEmbedFooter::new(
                "Well done, young traveller at getting denied from the club!",
            ))
            .color(0x00ff00),
    );

    private_channel
        .send_message(&discord.http(), msg.clone())
        .await?;

    ChannelId(crate::CONFIG.channels.mod_logs)
        .send_message(&discord.http(), msg)
        .await?;

    let request = reqwest::Client::new()
        .post(format!("https://catnip.metrobots.xyz/bots/{}/deny", bot_id))
        .query(&[("list_id", crate::CONFIG.metro.list_id.clone())])
        .query(&[("reviewer", bot_id)])
        .header("Authorization", crate::CONFIG.metro.secret.clone())
        .json(&Reason {
            reason: reason.to_string(),
        })
        .send()
        .await?;

    if request.status().is_success() {
        info!("Successfully denied bot {} on metro", bot_id);
        Ok(())
    } else {
        Err("Failed to deny bot on metro (but successful denial on IBL".into())
    }
}

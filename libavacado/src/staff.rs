/*
Implementation of common staff functions that can be shared between bot and API

Currently main actions are: approve, deny, vote reset (coming soon)

Smaller utilities specific to staff like add_action_log are also here
*/
use sqlx::PgPool;
use crate::types::Error;
use serde::Serialize;
use log::info;
use serenity::http::CacheHttp;

use serenity::model::id::{ChannelId, UserId};

#[derive(Serialize)]
struct Reason {
    reason: String,
}

/// Records a action log
pub async fn add_action_log(
    pool: &PgPool,
    bot_id: String,
    staff_id: String,
    reason: String,
    event_type: String,
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

/// Deny bot implementation
pub async fn deny_bot(
    discord: impl CacheHttp,
    pool: &PgPool,
    bot_id: String,
    staff_id: String,
    reason: String,
) -> Result<(), Error> {
    // The bot has way better onboarding, but this is a generic impl function so we need it
    let onboard_state = sqlx::query!(
        "SELECT staff_onboard_state FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    // We should never get this on bot, but maybe on website
    if onboard_state.staff_onboard_state != "complete" {
        return Err("onboarding_required".into());
    }

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, claimed = false WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(pool)
    .await?;

    let claimed = sqlx::query!(
        "SELECT type, claimed_by, owner, last_claimed FROM bots WHERE bot_id = $1",
        bot_id
    )
    .fetch_one(pool)
    .await?;

    if claimed.r#type != "pending" {
        return Err("Bot is not pending review?".into());
    }

    if claimed.claimed_by.is_none()
        || claimed.claimed_by.as_ref().unwrap().is_empty()
        || claimed.last_claimed.is_none()
    {
        return Err(
            format!(
                "<@{}> is not claimed? Do ``/claim`` to claim this bot first!",
                bot_id.clone()
            )
            .into());
    } 

    let start_time = chrono::offset::Utc::now();
    let last_claimed = claimed.last_claimed.unwrap();

    if (start_time - last_claimed).num_minutes() < 15 {
        return Err("Whoa there! You need to test this bot for at least 15 minutes (recommended: 20 minutes) before being able to approve/deny it!".into());
    }

    // Get main owner and modlogs
    let owner = UserId(claimed.owner.parse::<u64>()?);

    let modlogs = ChannelId(std::env::var("MODLOGS_CHANNEL")?.parse::<u64>()?);

    // Add action logs
    add_action_log(
        &pool,
        bot_id.to_string(),
        staff_id.to_string(),
        reason.to_string(),
        "deny".to_string(),
    )
    .await?;

    sqlx::query!(
        "UPDATE bots SET type = 'denied', claimed_by = NULL, claimed = false WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    let private_channel = owner.create_dm_channel(&discord).await?;

    private_channel
        .send_message(&discord.http(), |m| {
            m.embed(|e| {
                e.title("Bot Denied!")
                    .description(format!(
                        "<@{}> has denied <@{}>",
                        staff_id,
                        bot_id
                    ))
                    .field("Reason", reason.clone(), true)
                    .footer(|f| {
                        f.text("Well done, young traveller at getting denied from the club!")
                    })
                    .color(0x00ff00)
            })
        })
        .await?;

        modlogs
            .send_message(&discord.http(), |m| {
                m.embed(|e| {
                    e.title("__Bot Denied!__")
                        .field("Reason", &reason, true)
                        .field("Moderator", "<@".to_string()+&staff_id+">", true)
                        .field("Bot", "<@".to_string()+&bot_id+">", true)
                        .footer(|f| f.text("Sad life!"))
                        .color(0xFF0000)
                })
            })
            .await?;

        let request = reqwest::Client::new()
            .post(format!(
                "https://catnip.metrobots.xyz/bots/{}/deny",
                bot_id
            ))
            .query(&[("list_id", std::env::var("LIST_ID")?)])
            .query(&[("reviewer", bot_id.clone())])
            .header("Authorization", std::env::var("SECRET_KEY")?)
            .json(&Reason {
                reason: reason.clone(),
            })
            .send()
            .await?;

        if request.status().is_success() {
            info!("Successfully denied bot {} on metro", bot_id.clone());
            return Ok(());
        } else {
            return Err("Failed to deny bot on metro (but successful denial on IBL".into());
        }    
}
/*
Implementation of common staff functions that can be shared between bot and API

Currently main actions are: approve, deny, vote reset (coming soon)

Smaller utilities specific to staff like add_action_log are also here
*/
use crate::types::Error;
use log::info;
use serde::Serialize;
use serenity::http::CacheHttp;
use sqlx::PgPool;

use serenity::model::id::{ChannelId, GuildId, UserId};

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

pub async fn bot_owner_in_server(
    pool: &PgPool,
    discord: impl CacheHttp,
    bot_id: &str,
) -> Result<bool, Error> {
    // Get owners and additional owners
    let owners = sqlx::query!(
        "SELECT owner, additional_owners FROM bots WHERE bot_id = $1",
        bot_id
    )
    .fetch_one(pool)
    .await?;

    // Check if owner is in server ``MAIN_SERVER``
    let main_server = GuildId(std::env::var("MAIN_SERVER")?.parse::<u64>()?);

    let main_owner = owners.owner.parse::<u64>()?;

    let owner_in_server = discord
        .cache()
        .unwrap()
        .member_field(main_server, main_owner, |f| f.user.id);

    if owner_in_server.is_some() {
        return Ok(true);
    }

    // Check additional owners
    for owner in owners.additional_owners {
        let owner = owner.parse::<u64>();

        if owner.is_err() {
            continue;
        }

        let owner = owner.unwrap();

        let owner_in_server = discord
            .cache()
            .unwrap()
            .member_field(main_server, owner, |f| f.user.id);

        if owner_in_server.is_some() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Approve bot implementation
pub async fn approve_bot(
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
    if onboard_state.staff_onboard_state != "complete" {
        return Err("onboarding_required".into());
    }

    if reason.len() < 5 || reason.len() > 1998 {
        return Err("Reason is too short or too long".into());
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

    // Make sure a owner is in the server
    if !bot_owner_in_server(&pool, &discord, &bot_id).await? {
        return Err("The bot owner is not in the server".into());
    }

    if claimed.claimed_by.is_none()
        || claimed.claimed_by.as_ref().unwrap().is_empty()
        || claimed.last_claimed.is_none()
    {
        return Err(format!(
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

    add_action_log(
        pool,
        &bot_id,
        &staff_id,
        reason,
        "approve",
    )
    .await?;

    sqlx::query!(
        "UPDATE bots SET type = 'approved', claimed_by = NULL, claimed = false WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    // Get main owner and modlogs
    let owner = UserId(claimed.owner.parse::<u64>()?);

    let modlogs = ChannelId(std::env::var("MODLOGS_CHANNEL")?.parse::<u64>()?);

    let private_channel = owner.create_dm_channel(&discord).await?;

    private_channel
        .send_message(&discord.http(), |m| {
            m.embed(|e| {
                e.title("Bot Approved!")
                    .description(format!("<@{}> has approved <@{}>", staff_id, bot_id))
                    .field("Reason", reason.clone(), true)
                    .footer(|f| f.text("Well done, young traveller!"))
                    .color(0x00ff00)
            })
        })
        .await?;

    modlogs
        .send_message(&discord.http(), |m| {
            m.embed(|e| {
                e.title("__Bot Approved!__")
                    .field("Feedback", &reason, true)
                    .field("Moderator", "<@".to_string() + &staff_id + ">", true)
                    .field("Bot", "<@".to_string() + &bot_id + ">", true)
                    .footer(|f| f.text("Congratulations on your achievement!"))
                    .color(0x00ff00)
            })
        })
        .await?;

    let request = reqwest::Client::new()
        .post(format!(
            "https://catnip.metrobots.xyz/bots/{}/approve",
            bot_id
        ))
        .query(&[("list_id", std::env::var("LIST_ID")?)])
        .query(&[("reviewer", bot_id.clone())])
        .header("Authorization", std::env::var("SECRET_KEY")?)
        .json(&Reason {
            reason: reason.to_string(),
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
    if onboard_state.staff_onboard_state != "complete" {
        return Err("onboarding_required".into());
    }

    if reason.len() < 5 || reason.len() > 1998 {
        return Err("Reason is too short or too long".into());
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
        return Err(format!(
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
        bot_id,
        staff_id,
        reason,
        "deny",
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
                    .description(format!("<@{}> has denied <@{}>", staff_id, bot_id))
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
                    .field("Moderator", "<@".to_string() + &staff_id + ">", true)
                    .field("Bot", "<@".to_string() + &bot_id + ">", true)
                    .footer(|f| f.text("Sad life!"))
                    .color(0xFF0000)
            })
        })
        .await?;

    let request = reqwest::Client::new()
        .post(format!("https://catnip.metrobots.xyz/bots/{}/deny", bot_id))
        .query(&[("list_id", std::env::var("LIST_ID")?)])
        .query(&[("reviewer", bot_id.clone())])
        .header("Authorization", std::env::var("SECRET_KEY")?)
        .json(&Reason {
            reason: reason.to_string(),
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

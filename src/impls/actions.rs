use std::num::NonZeroU64;

use crate::config;
use poise::serenity_prelude::{
    builder::{CreateEmbed, CreateEmbedFooter, CreateMessage},
    model::id::ChannelId,
    GuildId, UserId, RoleId,
};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
struct MetroReason {
    reason: String,
}

use crate::impls::cache::CacheHttpImpl;
use crate::Error;

/// Records a action log
pub async fn add_action_log(
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
    event_type: &str,
) -> Result<(), Error> {
    if reason.len() < 5 || reason.len() > 1998 {
        return Err("Reason is too short or too long".into());
    }

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

pub async fn vote_reset_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    let staff_id_snow = UserId(staff_id.parse::<NonZeroU64>()?);

    if !config::CONFIG.owners.contains(&staff_id_snow.0) {
        return Err("You cannot reset votes unless you are owner".into());
    }

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

    add_action_log(pool, bot_id, staff_id, reason, "vote_reset").await?;

    sqlx::query!("UPDATE bots SET votes = 0 WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM votes WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__Bot Vote Reset!__")
            .field("Reason", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .field("Bot", "<@!".to_string() + bot_id + ">", true)
            .footer(CreateEmbedFooter::new("Sad life :("))
            .color(0xFF0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
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
    let staff_id_snow = UserId(staff_id.parse::<NonZeroU64>()?);

    if !config::CONFIG.owners.contains(&staff_id_snow.0) {
        return Err("You cannot reset votes unless you are owner".into());
    }

    // If bot_id is "all", reset all votes
    add_action_log(pool, &config::CONFIG.test_bot.to_string(), staff_id, reason, "vote_reset").await?;

    sqlx::query!("UPDATE bots SET votes = 0")
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM votes").execute(pool).await?;

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__All Votes Reset!__")
            .field("Reason", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .footer(CreateEmbedFooter::new("Sad life :("))
            .color(0xFF0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
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
    // Ensure user has iblhdev or hadmin
    let check = sqlx::query!("SELECT staff FROM users WHERE user_id = $1", staff_id)
        .fetch_one(pool)
        .await?;

    if !(check.staff) {
        return Err("You need to be a staff member to unverify bots".into());
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    add_action_log(pool, bot_id, staff_id, reason, "unverify").await?;

    sqlx::query!("UPDATE bots SET type = 'pending' WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__Bot Unverified For Futher Review!__")
            .field("Reason", reason, true)
            .field("Moderator", "<@".to_string() + staff_id + ">", true)
            .field("Bot", "<@!".to_string() + bot_id + ">", true)
            .footer(CreateEmbedFooter::new("Gonna be pending further review..."))
            .color(0xFF0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

/// Approve bot implementation
pub async fn approve_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<String, Error> {
    // The bot has way better onboarding, block in RPC
    let onboard_state = sqlx::query!(
        "SELECT staff, staff_onboard_state FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    if !onboard_state.staff {
        return Err("Only staff members may approve bots".into());
    }

    // We should never get this on bot, but maybe on website
    if onboard_state.staff_onboard_state != crate::impls::onboard_states::OnboardState::Completed.to_string() {
        return Err("onboarding_required".into());
    }

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(pool)
    .await?;

    let claimed = sqlx::query!(
        "SELECT type, claimed_by, last_claimed FROM bots WHERE bot_id = $1",
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

    // Find bot in testing server
    {
        let guild = discord
            .cache
            .guild(GuildId(crate::config::CONFIG.servers.testing))
            .ok_or("Failed to find guild")?;

        let member = guild.members.contains_key(&UserId(bot_id.parse()?));

        if !member {
            return Err("Bot is not in testing server. Please ensure this bot is in the testing server when approving. It will then be kicked by Arcadia when added to main server".into());
        }
    }

    let ping = super::utils::resolve_ping_user(bot_id, pool).await?;

    add_action_log(pool, bot_id, staff_id, reason, "approve").await?;

    sqlx::query!(
        "UPDATE bots SET type = 'approved', claimed_by = NULL WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    let bot_owners = super::utils::get_bot_members(bot_id, pool).await?;

    for owner in bot_owners {
        let owner_snow = UserId(owner.parse()?);

        // Add role to user
        discord.http
        .add_member_role(
            GuildId(config::CONFIG.servers.main),
            owner_snow,
            RoleId(config::CONFIG.roles.bot_developer),
            Some("Autorole due to bots owned"),
        )
        .await?;
    }

    let msg = CreateMessage::default()
        .content(format!("<@!{}>", ping))
        .embed(
            CreateEmbed::default()
                .title("Bot Approved!")
                .url(format!("{}/bots/{}", config::CONFIG.frontend_url, bot_id))
                .description(format!("<@!{}> has approved <@!{}>", staff_id, bot_id))
                .field("Feedback", reason, true)
                .field("Moderator", "<@!".to_string() + staff_id + ">", true)
                .field("Bot", "<@!".to_string() + bot_id + ">", true)
                .footer(CreateEmbedFooter::new("Well done, young traveller!"))
                .color(0x00ff00),
        );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    let invite_data = sqlx::query!("SELECT invite FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    Ok(invite_data.invite)
}

/// Deny bot implementation
pub async fn deny_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    // The bot has way better onboarding, but this is a generic impl function so we need it
    let onboard_state = sqlx::query!(
        "SELECT staff, staff_onboard_state FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    if !onboard_state.staff {
        return Err("Only staff members may deny bots".into());
    }

    if onboard_state.staff_onboard_state != crate::impls::onboard_states::OnboardState::Completed.to_string() {
        return Err("You need to complete onboarding to continue!".into());
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

    let ping = super::utils::resolve_ping_user(bot_id, pool).await?;

    // Add action logs
    add_action_log(pool, bot_id, staff_id, reason, "deny").await?;

    sqlx::query!(
        "UPDATE bots SET type = 'denied', claimed_by = NULL WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    let msg = CreateMessage::new().content(format!("<@!{}>", ping)).embed(
        CreateEmbed::default()
            .title("Bot Denied!")
            .url(format!("{}/bots/{}", config::CONFIG.frontend_url, bot_id))
            .description(format!("<@{}> has denied <@{}>", staff_id, bot_id))
            .field("Reason", reason, true)
            .field("Moderator", "<@!".to_string() + staff_id + ">", true)
            .field("Bot", "<@!".to_string() + bot_id + ">", true)
            .footer(CreateEmbedFooter::new(
                "Well done, young traveller at getting denied from the club!",
            ))
            .color(0x00ff00),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

pub async fn premium_add_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
    time_period: i32, /* in hours */
) -> Result<(), Error> {
    // Ensure user has iblhdev or hadmin
    let check = sqlx::query!(
        "SELECT iblhdev, hadmin FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    if !(check.iblhdev || check.hadmin) {
        return Err(
            "You need `Head Staff Manager` or `Head Developer` to add premium to bots".into(),
        );
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    add_action_log(
        pool,
        bot_id,
        staff_id,
        &(reason.to_string() + ": " + &time_period.to_string()),
        "premium_add",
    )
    .await?;

    // Set premium_period_length which is a postgres interval
    sqlx::query!(
        "UPDATE bots SET start_premium_period = NOW(), premium_period_length = make_interval(hours => $1), premium = true WHERE bot_id = $2",
        time_period,
        bot_id
    )
    .execute(pool)
    .await?;

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Premium Added!")
            .description(format!(
                "<@{}> has added premium to <@{}> for {} hours",
                staff_id, bot_id, time_period
            ))
            .field("Reason", reason, true)
            .footer(CreateEmbedFooter::new(
                "Well done, young traveller! Use it wisely...",
            ))
            .color(0x00ff00),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

pub async fn certify_remove_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    // Ensure user has iblhdev or hadmin
    let check = sqlx::query!(
        "SELECT iblhdev, hadmin FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    if !(check.iblhdev || check.hadmin) {
        return Err("You need `Head Staff Manager` or `Head Developer` to uncertify bots".into());
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    add_action_log(pool, bot_id, staff_id, reason, "certify_remove_bot").await?;

    // Set premium_period_length which is a postgres interval
    sqlx::query!(
        "UPDATE bots SET type = 'approved' WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Bot Uncertified!")
            .description(format!("<@{}> has uncertified <@{}>", staff_id, bot_id))
            .field("Reason", reason, true)
            .footer(CreateEmbedFooter::new(
                "Uh oh, looks like you've been naughty...",
            ))
            .color(0xff0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

pub async fn premium_remove_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    // Ensure user has iblhdev or hadmin
    let check = sqlx::query!(
        "SELECT iblhdev, hadmin FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    if !(check.iblhdev || check.hadmin) {
        return Err(
            "You need `Head Staff Manager` or `Head Developer` to remove premium from bots".into(),
        );
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    add_action_log(pool, bot_id, staff_id, reason, "premium_remove").await?;

    // Set premium_period_length which is a postgres interval
    sqlx::query!("UPDATE bots SET premium = false WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Premium Removed!")
            .description(format!(
                "<@{}> has removed premium from <@{}>",
                staff_id, bot_id
            ))
            .field("Reason", reason, true)
            .footer(CreateEmbedFooter::new(
                "Well done, young traveller. Sad to see you go...",
            ))
            .color(0xFF0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

pub async fn vote_ban_add_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    // Ensure user has iblhdev or hadmin
    let check = sqlx::query!(
        "SELECT iblhdev, hadmin FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    if !(check.iblhdev || check.hadmin) {
        return Err("You need `Head Staff Manager` or `Head Developer` to edit votebans".into());
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    add_action_log(pool, bot_id, staff_id, reason, "vote_ban_add").await?;

    // Set premium_period_length which is a postgres interval
    sqlx::query!(
        "UPDATE bots SET vote_banned = true WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Vote Ban Edit!")
            .description(format!(
                "<@{}> has set the vote ban on <@{}>",
                staff_id, bot_id,
            ))
            .field("Reason", reason, true)
            .footer(CreateEmbedFooter::new(
                "Remember: don't abuse our services!",
            ))
            .color(0xFF0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

pub async fn force_bot_remove(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
    kick: bool,
) -> Result<(), Error> {
    // Ensure user has iblhdev or hadmin
    let check = sqlx::query!(
        "SELECT iblhdev, hadmin FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    if !(check.iblhdev || check.hadmin) {
        return Err(
            "You need `Head Staff Manager` or `Head Developer` to forcibly remove a bit".into(),
        );
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    let bot_id_snow = bot_id.parse::<NonZeroU64>()?;

    if crate::config::CONFIG.protected_bots.contains(&bot_id_snow) && kick {
        return Err("You can't force delete this bot with 'kick' enabled!".into());
    }

    add_action_log(pool, bot_id, staff_id, reason, "force_bot_remove").await?;

    // Set premium_period_length which is a postgres interval
    sqlx::query!("DELETE FROM bots WHERE bot_id = $1", bot_id)
        .execute(pool)
        .await?;

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Bot Force Deleted!")
            .description(format!(
                "<@{}> has force-removed <@{}> for violating our rules or Discord ToS",
                staff_id, bot_id,
            ))
            .field("Reason", reason, true)
            .footer(CreateEmbedFooter::new(
                "Remember: don't abuse our services!",
            ))
            .color(0xFF0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    if kick {
        // Check that the bot is in the server
        let bot = discord.cache.member_field(
            GuildId(crate::config::CONFIG.servers.main),
            UserId(bot_id_snow),
            |m| m.user.name.clone(),
        );

        if bot.is_some() {
            GuildId(crate::config::CONFIG.servers.main)
                .member(&discord, UserId(bot_id.parse()?))
                .await?
                .kick_with_reason(&discord, &(staff_id.to_string() + ":" + reason))
                .await?;
        }
    }

    Ok(())
}

pub async fn vote_ban_remove_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
) -> Result<(), Error> {
    // Ensure user has iblhdev or hadmin
    let check = sqlx::query!(
        "SELECT iblhdev, hadmin FROM users WHERE user_id = $1",
        staff_id
    )
    .fetch_one(pool)
    .await?;

    if !(check.iblhdev || check.hadmin) {
        return Err("You need `Head Staff Manager` or `Head Developer` to edit votebans".into());
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    add_action_log(pool, bot_id, staff_id, reason, "vote_ban_remove").await?;

    sqlx::query!(
        "UPDATE bots SET vote_banned = false WHERE bot_id = $1",
        bot_id
    )
    .execute(pool)
    .await?;

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Vote Ban Removed!")
            .description(format!(
                "<@{}> has removed the vote ban on <@{}>",
                staff_id, bot_id,
            ))
            .field("Reason", reason, true)
            .footer(CreateEmbedFooter::new(
                "Remember: don't abuse our services!",
            ))
            .color(0xFF0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

pub async fn vote_count_set_bot(
    discord: &CacheHttpImpl,
    pool: &PgPool,
    bot_id: &str,
    staff_id: &str,
    reason: &str,
    count: i32,
) -> Result<(), Error> {
    let staff_id_snow = UserId(staff_id.parse::<NonZeroU64>()?);

    if !config::CONFIG.owners.contains(&staff_id_snow.0) {
        return Err("You cannot reset votes unless you are owner".into());
    }

    // Ensure the bot actually exists
    let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await?;

    if bot.count.unwrap_or_default() == 0 {
        return Err("Bot does not exist".into());
    }

    add_action_log(pool, bot_id, staff_id, reason, "vote_count_set").await?;

    // Set premium_period_length which is a postgres interval
    sqlx::query!(
        "UPDATE bots SET votes = $2 WHERE bot_id = $1",
        bot_id,
        count
    )
    .execute(pool)
    .await?;

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Vote Count Updated!")
            .description(format!(
                "<@{}> has force-updated the vote count of <@{}>",
                staff_id, bot_id,
            ))
            .field("Reason", reason, true)
            .field("New Vote Count", count.to_string(), true)
            .footer(CreateEmbedFooter::new(
                "Remember: don't abuse our services!",
            ))
            .color(0xFF0000),
    );

    ChannelId(crate::config::CONFIG.channels.mod_logs)
        .send_message(&discord, msg)
        .await?;

    Ok(())
}

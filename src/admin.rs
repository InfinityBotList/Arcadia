use crate::checks;
use crate::impls;
use crate::Context;
use crate::Error;

use poise::serenity_prelude::CreateActionRow;
use poise::serenity_prelude::CreateButton;
use poise::serenity_prelude::CreateMessage;
use poise::serenity_prelude::User;
use poise::CreateReply;

use poise::serenity_prelude as serenity;

/// Onboarding base command
#[poise::command(
    category = "Admin",
    prefix_command,
    slash_command,
    guild_cooldown = 10,
    subcommands("approveonboard", "denyonboard", "resetonboard",)
)]
pub async fn onboardman(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Some available options are ``onboardman approve`` etc.")
        .await?;
    Ok(())
}

/// Allows managers to onboard users
#[poise::command(
    rename = "approve",
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin",
    check = "checks::staff_server"
)]
pub async fn approveonboard(
    ctx: Context<'_>,
    #[description = "The staff id"] member: serenity::User,
) -> Result<(), Error> {
    let data = ctx.data();

    // Check onboard state of user
    let onboard_state = sqlx::query!(
        "SELECT staff_onboard_state FROM users WHERE user_id = $1",
        member.id.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    if onboard_state.staff_onboard_state
        != crate::onboarding::OnboardState::PendingManagerReview.as_str()
        && onboard_state.staff_onboard_state != crate::onboarding::OnboardState::Denied.as_str()
    {
        return Err(format!(
            "User is not pending manager review and currently has state of: {}",
            onboard_state.staff_onboard_state
        )
        .into());
    }

    // Update onboard state of user
    sqlx::query!(
        "UPDATE users SET staff_onboard_state = $1 WHERE user_id = $2",
        crate::onboarding::OnboardState::Completed.as_str(),
        member.id.to_string()
    )
    .execute(&data.pool)
    .await?;

    // DM user that they have been approved
    let _ = member.dm(
        &ctx.discord().http,
        CreateMessage::new()
        .content("Your onboarding request has been approved. You may now begin approving/denying bots") 
    ).await?;

    ctx.say("Onboarding request approved!").await?;

    Ok(())
}

/// Denies onboarding requests
#[poise::command(
    rename = "deny",
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin",
    check = "checks::staff_server"
)]
pub async fn denyonboard(
    ctx: crate::Context<'_>,
    #[description = "The staff id"] user: serenity::User,
) -> Result<(), Error> {
    let data = ctx.data();

    // Check onboard state of user
    let onboard_state = sqlx::query!(
        "SELECT staff_onboard_state FROM users WHERE user_id = $1",
        user.id.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    if onboard_state.staff_onboard_state
        != crate::onboarding::OnboardState::PendingManagerReview.as_str()
    {
        return Err(format!(
            "User is not pending manager review and currently has state of: {}",
            onboard_state.staff_onboard_state
        )
        .into());
    }

    // Update onboard state of user
    sqlx::query!(
        "UPDATE users SET staff_onboard_state = $1 WHERE user_id = $2",
        crate::onboarding::OnboardState::Denied.as_str(),
        user.id.to_string()
    )
    .execute(&data.pool)
    .await?;

    // DM user that they have been denied
    let _ = user.dm(&ctx.discord().http, CreateMessage::new().content("Your onboarding request has been denied. Please contact a manager for more information")).await?;

    ctx.say("Onboarding request denied!").await?;

    Ok(())
}

/// Resets a onboarding to force a new one
#[poise::command(
    rename = "reset",
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin",
    check = "checks::staff_server"
)]
pub async fn resetonboard(
    ctx: crate::Context<'_>,
    #[description = "The staff id"] user: serenity::User,
) -> Result<(), Error> {
    let data = ctx.data();

    let builder = CreateReply::new()
        .content("Are you sure you wish to reset this user's onboard state and force them to redo onboarding?")
        .components(
            vec![
                CreateActionRow::Buttons(
                    vec![
                        CreateButton::new("continue").label("Continue").style(serenity::ButtonStyle::Primary),
                        CreateButton::new("cancel").label("Cancel").style(serenity::ButtonStyle::Danger),
                    ]
                )
            ]
        );

    let mut msg = ctx.send(builder.clone()).await?.into_message().await?;

    let interaction = msg
        .await_component_interaction(ctx.discord())
        .author_id(ctx.author().id)
        .await;

    msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![]))
        .await?; // remove buttons after button press

    let pressed_button_id = match &interaction {
        Some(m) => &m.data.custom_id,
        None => {
            ctx.say("You didn't interact in time").await?;
            return Ok(());
        }
    };

    if pressed_button_id == "cancel" {
        ctx.say("Cancelled").await?;
        return Ok(());
    }

    // Update onboard state of user
    sqlx::query!(
        "UPDATE users SET staff_onboard_guild = NULL, staff_onboard_state = $1, staff_onboard_last_start_time = NOW() WHERE user_id = $2",
        crate::onboarding::OnboardState::Pending.as_str(),
        user.id.to_string()
    )
    .execute(&data.pool)
    .await?;

    // DM user that they have been force reset
    let _ = user.dm(&ctx.discord().http, CreateMessage::new().content("Your onboarding request has been force reset. Please contact a manager for more information. You will, in most cases, need to redo onboarding")).await?;

    ctx.say("Onboarding request reset!").await?;

    Ok(())
}

/// Bot management command
#[poise::command(
    category = "Admin",
    prefix_command,
    slash_command,
    guild_cooldown = 10,
    subcommands("botunverify", "botpremiumadd", "botpremiumdel", "botvotereset", "botvoteresetall", "botvotebanadd", "botvotebandel")
)]
pub async fn botman(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("See /help botman for more info").await?;
    Ok(())
}

/// Resets the votes of a bot
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn botvotereset(
    ctx: crate::Context<'_>,
    #[description = "The bots ID"] bot: User,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    let data = ctx.data();

    impls::actions::vote_reset_bot(
        &data.cache_http,
        &data.pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("This bots votes have been reset!").await?;

    Ok(())
}

/// Resets the votes of all bots
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn botvoteresetall(
    ctx: crate::Context<'_>,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    let data = ctx.data();

    impls::actions::vote_reset_all_bot(
        &data.cache_http,
        &data.pool,
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("All bot votes have been reset!").await?;

    Ok(())
}

/// Unverifies a bot for further review
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn botunverify(
    ctx: crate::Context<'_>,
    #[description = "The bots ID"] bot: User,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    let data = ctx.data();

    impls::actions::unverify_bot(
        &data.cache_http,
        &data.pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("This bot has been unverified!").await?;

    Ok(())
}

#[derive(poise::ChoiceParameter)]
pub enum TimePeriodUnit {
    #[name = "Years"]
    Years,
    #[name = "Days"]
    Days,
    #[name = "Hours"]
    Hours,
}

/// Adds premium to a bot
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn botpremiumadd(
    ctx: crate::Context<'_>,
    #[description = "The bots ID"] bot: User,
    #[description = "The reason"] reason: String,
    #[description = "The time period (in days or hours)"] time_period: i32,
    #[description = "The time period unit (days, hours etc)"] time_unit: TimePeriodUnit,
) -> Result<(), crate::Error> {
    let data = ctx.data();

    impls::actions::premium_add_bot(
        &data.cache_http,
        &data.pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
        match time_unit {
            TimePeriodUnit::Years => time_period * 365 * 24,
            TimePeriodUnit::Days => time_period * 24,
            TimePeriodUnit::Hours => time_period,
        },
    )
    .await?;

    ctx.say("This bot has been added to premium successfully!")
        .await?;

    Ok(())
}

/// Removes premium from a bot
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn botpremiumdel(
    ctx: crate::Context<'_>,
    #[description = "The bots ID"] bot: User,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    let data = ctx.data();

    impls::actions::premium_remove_bot(
        &data.cache_http,
        &data.pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("This bot has been removed from premium successfully!")
        .await?;

    Ok(())
}

/// Bans or unbans a bot from votes
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn botvotebanadd(
    ctx: crate::Context<'_>,
    #[description = "The bots ID"] bot: User,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    let data = ctx.data();

    impls::actions::vote_ban_add_bot(
        &data.cache_http,
        &data.pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("This bot has been vote banned!")
        .await?;

    Ok(())
}

/// Bans or unbans a bot from votes
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn botvotebandel(
    ctx: crate::Context<'_>,
    #[description = "The bots ID"] bot: User,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    let data = ctx.data();

    impls::actions::vote_ban_remove_bot(
        &data.cache_http,
        &data.pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("This bot has been un-vote banned!")
        .await?;

    Ok(())
}

/*

/// Unlocks RPC for a one hour time period, is logged
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn rpcunlock(
    ctx: crate::Context<'_>,
    #[description = "Purpose"] purpose: String,
) -> Result<(), Error> {
    let nonce = impls::crypto::gen_random(5);

    let warn_embed = {
        CreateEmbed::new()
        .title(":warning: Warning")
        .description(
            format!("**You are about to unlock full access to the RPC API for one hour on your account (required by some parts of our staff panel)**

While RPC is unlocked, any leaks have a higher change in in data being destroyed and mass-nukes to potentially occur although the API does protect against it using ratelimits!

To continue, please click the `Unlock` button and input ``{}`` in the next 30 seconds OR use bot commands instead (where permitted).
            ", 
            nonce)
        )
    };

    Ok(())
}
*/
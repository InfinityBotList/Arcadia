use crate::Context;
use crate::Error;
use crate::_checks as checks;

use poise::CreateReply;
use poise::serenity_prelude::CreateActionRow;
use poise::serenity_prelude::CreateButton;
use poise::serenity_prelude::CreateMessage;
use poise::serenity_prelude::User;

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

    if onboard_state.staff_onboard_state != "pending-manager-review"
        && onboard_state.staff_onboard_state != "denied"
    {
        return Err(format!(
            "User is not pending manager review and currently has state of: {}",
            onboard_state.staff_onboard_state
        )
        .into());
    }

    // Update onboard state of user
    sqlx::query!(
        "UPDATE users SET staff_onboard_state = 'complete' WHERE user_id = $1",
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

    if onboard_state.staff_onboard_state != "pending-manager-review" {
        return Err(format!(
            "User is not pending manager review and currently has state of: {}",
            onboard_state.staff_onboard_state
        )
        .into());
    }

    // Update onboard state of user
    sqlx::query!(
        "UPDATE users SET staff_onboard_state = 'denied' WHERE user_id = $1",
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

    let mut msg = ctx
        .send(builder.clone())
        .await?
        .into_message()
        .await?;

    let interaction = msg
        .component_interaction_collector(ctx.discord())
        .author_id(ctx.author().id)
        .collect_single()
        .await;

    msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![])).await?; // remove buttons after button press

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
        "UPDATE users SET staff_onboard_state = 'pending', staff_onboard_last_start_time = NOW() WHERE user_id = $1",
        user.id.to_string()
    )
    .execute(&data.pool)
    .await?;

    // DM user that they have been force reset
    let _ = user.dm(&ctx.discord().http, CreateMessage::new().content("Your onboarding request has been force reset. Please contact a manager for more information. You will, in most cases, need to redo onboarding")).await?;

    ctx.say("Onboarding request reset!").await?;

    Ok(())
}


#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn votereset(
    ctx: crate::Context<'_>,
    #[description = "The bots ID"] bot: User,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    libavacado::manage::vote_reset(
        &ctx.discord(),
        &ctx.data().pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await
}

#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_hdev_hadmin"
)]
pub async fn voteresetall(
    ctx: crate::Context<'_>,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    libavacado::manage::vote_reset_all(
        &ctx.discord(),
        &ctx.data().pool,
        &ctx.author().id.to_string(),
        &reason,
    )
    .await
}

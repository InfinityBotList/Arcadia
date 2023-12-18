use std::io::Write;

use crate::checks;
use crate::Context;
use crate::Error;
use poise::serenity_prelude::UserId;

#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server",
    check = "checks::is_staff"
)]
pub async fn uninvitedbots(ctx: Context<'_>) -> Result<(), Error> {
    let subject_rows = sqlx::query!(
        "SELECT bot_id, uptime, total_uptime FROM bots WHERE type = 'approved' OR type = 'certified'"
    )
    .fetch_all(&ctx.data().pool)
    .await?;

    let mut bad_ids = Vec::new();

    for row in subject_rows {
        match row.bot_id.parse::<UserId>() {
            Ok(id) => match ctx.cache().member(crate::config::CONFIG.servers.main, id) {
                Some(_) => continue,
                None => {
                    bad_ids.push(id.to_string());
                }
            },
            Err(_) => continue,
        }
    }

    log::error!("Bad ids: {:?}", bad_ids);

    // Get the first 10 bots
    let first_bots = bad_ids
        .iter()
        .take(10)
        .map(|x| x.to_string())
        .collect::<Vec<String>>();

    let mut msg = "".to_string();

    for bot in first_bots {
        msg.push_str(&format!("{id} https://discord.com/oauth2/authorize?client_id={id}&permissions=0&scope=bot%20applications.commands\n", id=bot));
    }

    msg.push_str(&format!("**Total Len:** {}", bad_ids.len()));

    ctx.say(msg).await?;

    Ok(())
}

/// Protects a deploy
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server",
    check = "checks::is_staff"
)]
pub async fn protectdeploy(
    ctx: Context<'_>,
    #[description = "Reason"] reason: String,
) -> Result<(), Error> {
    if !crate::config::CONFIG.owners.contains(&ctx.author().id) {
        ctx.say("Only owners can update the main site").await?;
        return Ok(());
    }

    let mut admin_meta_file = std::fs::File::create(".protect-deploy")?;

    admin_meta_file.write_all(reason.as_bytes())?;

    ctx.say("Deploy protected").await?;

    Ok(())
}

/// Unprotects a deploy
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server",
    check = "checks::is_staff"
)]
pub async fn unprotectdeploy(ctx: Context<'_>) -> Result<(), Error> {
    if !crate::config::CONFIG.owners.contains(&ctx.author().id) {
        ctx.say("Only owners can update the main site").await?;
        return Ok(());
    }

    let file_exists = std::path::Path::new(".protect-deploy").exists();

    if file_exists {
        std::fs::remove_file(".protect-deploy")?;
    }

    ctx.say("Deploy unprotected").await?;

    Ok(())
}

/// Updates the production build of the site. Owner only
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server",
    check = "checks::is_staff"
)]
pub async fn updprod(ctx: Context<'_>) -> Result<(), Error> {
    if !crate::config::CONFIG.owners.contains(&ctx.author().id) {
        ctx.say("Only owners can update the main site").await?;
        return Ok(());
    }

    ctx.say("``updprod`` has been moved to https://sysmanage.infinitybots.gg > Custom Actions")
        .await?;

    Ok(())
}

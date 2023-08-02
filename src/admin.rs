use std::io::Write;
use std::num::NonZeroU64;

use crate::checks;
use crate::Context;
use crate::Error;
use crate::rpc::server::KeychainData;
use moka::future::ConcurrentCacheExt;
use poise::serenity_prelude::ButtonStyle;
use poise::serenity_prelude::CacheHttp;
use poise::serenity_prelude::CreateActionRow;
use poise::serenity_prelude::CreateButton;
use poise::serenity_prelude::CreateEmbed;
use poise::CreateReply;
use serde_json::json;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::GuildId;
use ::serenity::all::Attachment;
use ::serenity::builder::CreateAttachment;
use strum::VariantNames;

/// Unlocks RPC for a 10 minutes, is logged
#[poise::command(
    category = "Admin",
    track_edits,
    slash_command,
    check = "checks::staff_server",
    check = "checks::is_staff"
)]
pub async fn rpcidentify(
    ctx: Context<'_>,
    #[description = "Purpose"] purpose: String,
    #[description = "Which methods to enable"] methods: Option<String>,
    #[description = "Maximum Uses"] max_uses: Option<u8>
) -> Result<(), Error> {
    // Parse methods
    let methods = methods.unwrap_or("BotClaim,BotUnclaim,BotApprove,BotDeny".into());

    let methods = methods.replace(' ', ",");
    let methods = methods.split(',');

    let mut allowed_methods = vec![];

    for method in methods {
        if method.is_empty() {
            continue
        }

        if !crate::rpc::core::RPCMethod::VARIANTS.contains(&method) {
            return Err(
                format!(
                    "Invalid RPC Method: {}", method
                ).into()
            );
        }

        allowed_methods.push(method);
    }

    let max_uses = max_uses.unwrap_or(3);

    if max_uses > 10 {
        return Err(
            "Maximum uses cannot be greater than 10".into()
        );
    }

    let warn_embed = {
        CreateEmbed::new()
        .title(":warning: Warning")
        .description(
            format!("**You are about to unlock access to the RPC API for 10 minutes on your account (required by some parts of our staff panel)**

While RPC is unlocked, any leaks or bugs have a higher change in leading to data being destroyed and mass-nukes to potentially occur although the API does try to protect against it using ratelimits etc.!

To continue, please click the `Unlock` button OR instead, (PREFERRED) just use bot commands instead (where permitted).

**Given Reason:** {}
**Allowed Methods:** {:?}
**Maximum Uses Of Identity:** {}
            ", 
            purpose,
            allowed_methods,
            max_uses
        ))
        .color(0xFF0000)
    };

    let msg = ctx
        .send(
            CreateReply::new()
                .embed(warn_embed)
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new("a:unlock")
                        .style(ButtonStyle::Primary)
                        .label("Unlock"),
                    CreateButton::new("a:cancel")
                        .style(ButtonStyle::Danger)
                        .label("Cancel"),
                ])]),
        )
        .await?
        .into_message()
        .await?;

    let interaction = msg
        .await_component_interaction(ctx.discord())
        .author_id(ctx.author().id)
        .await;

    if let Some(item) = interaction {
        let custom_id = &item.data.custom_id;

        if custom_id == "a:cancel" {
            item.delete_response(ctx.discord()).await?;
        } else if custom_id == "a:unlock" {
            item.defer_ephemeral(&ctx.discord()).await?;

            // Check number of current RPC sessions
            crate::rpc::server::RPC_KEYCHAIN.sync();
            
            if crate::rpc::server::RPC_KEYCHAIN.entry_count() > 10 {
                item.create_followup(
                    &ctx.discord(),
                    serenity::CreateInteractionResponseFollowup::default()
                    .content("Kittycat Security Patrol: Too many RPC sessions are currently active, please try again later.")
                )
                .await?;

                return Ok(());
            }

            // Create a RPC identity
            let rpc_identity = "Bluejay$V1:".to_string()+&crate::impls::crypto::gen_random(65535);

            sqlx::query!(
                "INSERT INTO staff_general_logs (user_id, action, data) VALUES ($1, $2, $3)",
                ctx.author().id.to_string(),
                "rpc_unlock",
                json!({
                    "reason": purpose,
                    "allowed_methods": allowed_methods,
                    "max_uses": max_uses,
                })
            )
            .execute(&ctx.data().pool)
            .await?;

            sqlx::query!(
                "UPDATE users SET staff_rpc_last_verify = NOW() WHERE user_id = $1",
                ctx.author().id.to_string()
            )
            .execute(&ctx.data().pool)
            .await?;

            let filehash = crate::impls::crypto::gen_random(12);

            item.create_followup(
                &ctx.discord(),
                serenity::CreateInteractionResponseFollowup::default()
                .content("Kittycat Security Patrol: Please find attached the code to *that thing*...")
                .add_file(
                    CreateAttachment::bytes(
                        rpc_identity.as_bytes().to_vec(),
                        "rpc_identity-".to_string()+&filehash,
                    )
                )
            )
            .await?;

            // save the identity
            crate::rpc::server::RPC_KEYCHAIN.insert(rpc_identity, KeychainData {
                user_id: ctx.author().id.to_string(),
                allowed_methods: allowed_methods.into_iter().map(|x| x.to_string()).collect(),
                max_uses,
                used: 0,
                reason: purpose,
            }).await;
        }
    }

    Ok(())
}

/// Locks RPC
#[poise::command(category = "Admin", track_edits, prefix_command, slash_command)]
pub async fn rpclock(
    ctx: Context<'_>,
    #[description = "RPC Identity File"] identity: Attachment,
) -> Result<(), Error> {
    ctx.defer().await?;

    // Download the file
    let file = identity.download().await?;

    // Parse the file
    let file = String::from_utf8(file)?;

    // Remove identity from moka
    crate::rpc::server::RPC_KEYCHAIN.remove(&file).await;

    ctx.say("RPC has been locked").await?;

    Ok(())
}

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
        match row.bot_id.parse::<NonZeroU64>() {
            Ok(id) => {
                match ctx
                    .cache_and_http()
                    .cache()
                    .ok_or("Error finding main server")?
                    .member_field(GuildId(crate::config::CONFIG.servers.main), id, |m| {
                        m.user.id
                    }) {
                    Some(_) => continue,
                    None => {
                        bad_ids.push(id.to_string());
                    }
                }
            }
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
    if !crate::config::CONFIG.owners.contains(&ctx.author().id.0) {
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
    if !crate::config::CONFIG.owners.contains(&ctx.author().id.0) {
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
    if !crate::config::CONFIG.owners.contains(&ctx.author().id.0) {
        ctx.say("Only owners can update the main site").await?;
        return Ok(());
    }

    ctx.say("``updprod`` has been moved to https://sysmanage.infinitybots.gg > Custom Actions").await?;

    Ok(())
}

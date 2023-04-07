use std::num::NonZeroU64;

use crate::checks;
use crate::impls::actions::add_action_log;
use crate::Context;
use crate::Error;
use poise::serenity_prelude::ButtonStyle;
use poise::serenity_prelude::CacheHttp;
use poise::serenity_prelude::CreateActionRow;
use poise::serenity_prelude::CreateButton;
use poise::serenity_prelude::CreateEmbed;
use poise::serenity_prelude::CreateInteractionResponseMessage;
use poise::CreateReply;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::GuildId;

/// Unlocks RPC for a 10 minutes, is logged
#[poise::command(
    category = "Admin",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server",
    check = "checks::is_staff"
)]
pub async fn rpcunlock(
    ctx: Context<'_>,
    #[description = "Purpose"] purpose: String,
) -> Result<(), Error> {
    let warn_embed = {
        CreateEmbed::new()
        .title(":warning: Warning")
        .description(
            format!("**You are about to unlock full access to the RPC API for 10 minutes on your account (required by some parts of our staff panel)**

While RPC is unlocked, any leaks or bugs have a higher change in leading to data being destroyed and mass-nukes to potentially occur although the API does try to protect against it using ratelimits etc.!

To continue, please click the `Unlock` button OR instead, (PREFERRED) just use bot commands instead (where permitted).

**Given Reason:** {}
            ", 
            purpose)
        )
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
            add_action_log(
                &ctx.data().pool,
                &crate::config::CONFIG.test_bot.to_string(),
                &ctx.author().id.to_string(),
                &purpose,
                "rpc_unlock",
            )
            .await?;

            sqlx::query!(
                "UPDATE users SET staff_rpc_last_verify = NOW() WHERE user_id = $1",
                ctx.author().id.to_string()
            )
            .execute(&ctx.data().pool)
            .await?;

            item.create_response(
                &ctx.discord(),
                serenity::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default().content("RPC unlocked"),
                ),
            )
            .await?;
        }
    }

    Ok(())
}

/// Locks RPC
#[poise::command(category = "Admin", track_edits, prefix_command, slash_command)]
pub async fn rpclock(ctx: Context<'_>) -> Result<(), Error> {
    sqlx::query!(
        "UPDATE users SET staff_rpc_last_verify = NOW() - interval '1 hour' WHERE user_id = $1",
        ctx.author().id.to_string()
    )
    .execute(&ctx.data().pool)
    .await?;

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
                match ctx.cache_and_http().cache().ok_or("Error finding main server")?.member_field(GuildId(crate::config::CONFIG.servers.main), id, |m| m.user.id) {
                    Some(_) => {
                        continue
                    }
                    None => {
                        bad_ids.push(id.to_string());
                    }
                }
            }
            Err(_) => {
                continue
            }
        }
    }
    
    log::error!("Bad ids: {:?}", bad_ids);

    // Get the first 10 bots
    let first_bots = bad_ids.iter().take(10).map(|x| x.to_string()).collect::<Vec<String>>();

    let mut msg = "".to_string();

    for bot in first_bots {
        msg.push_str(&format!("{id} https://discord.com/oauth2/authorize?client_id={id}&permissions=0&scope=bot%20applications.commands\n", id=bot));
    }

    msg.push_str(&format!("**Total Len:** {}", bad_ids.len()));

    ctx.say(msg).await?;

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

    // Delete the production branch using github api and github_pat
    let client = reqwest::Client::new();

    ctx.say("Deleting old `production` branch").await?;

    // Disable enforce_admin
    let res = client
        .delete(format!(
            "https://api.github.com/repos/{}/branches/production/protection/enforce_admins",
            crate::config::CONFIG.github_repo,
        ))
        .basic_auth(
            &crate::config::CONFIG.github_username,
            Some(&crate::config::CONFIG.github_pat),
        )
        .header("User-Agent", &crate::config::CONFIG.github_username)
        .send()
        .await?;

    if res.status() != 204 && res.status() != 404 {
        let body = res.text().await?;
        ctx.say(format!("Failed to remove enforce production branch protection rule: {}.", body)).await?;
        return Ok(());
    }        

    // Remove branch protection
    let res = client
    .delete(format!(
        "https://api.github.com/repos/{}/branches/production/protection",
        crate::config::CONFIG.github_repo,
    ))
    .basic_auth(
        &crate::config::CONFIG.github_username,
        Some(&crate::config::CONFIG.github_pat),
    )
    .header("User-Agent", &crate::config::CONFIG.github_username)
    .send()
    .await?;

    if res.status() != 204 && res.status() != 404 {
        let body = res.text().await?;
        ctx.say(format!("Failed to remove production branch protection: {}", body)).await?;
        return Ok(());
    }

    let res = client
        .delete(format!(
            "https://api.github.com/repos/{}/git/refs/heads/production",
            crate::config::CONFIG.github_repo,
        ))
        .basic_auth(
            &crate::config::CONFIG.github_username,
            Some(&crate::config::CONFIG.github_pat),
        )
        .header("User-Agent", &crate::config::CONFIG.github_username)
        .send()
        .await?;

    if res.status() == 422 {
        let body = res.text().await?;
        ctx.say(format!("Ignoring error 422 (branch not found): {}", body)).await?;
    } else if res.status() != 204 {
        ctx.say(format!(
            "Failed to delete production branch. Got status code: {} and resp: {}",
            res.status(),
            res.text().await?
        ))
        .await?;
        return Ok(());
    }

    ctx.say("Creating new `production` branch").await?;

    // Get SHA of master branch
    let res = client
        .get(format!(
            "https://api.github.com/repos/{}/git/refs/heads/master",
            crate::config::CONFIG.github_repo,
        ))
        .basic_auth(
            &crate::config::CONFIG.github_username,
            Some(&crate::config::CONFIG.github_pat),
        )
        .header("User-Agent", &crate::config::CONFIG.github_username)
        .send()
        .await?;

    if res.status() != 200 {
        ctx.say(format!(
            "Failed to fetch master branch. Got status code: {} and resp: {}",
            res.status(),
            res.text().await?
        ))
        .await?;
        return Ok(());
    }

    let sha = res.json::<serde_json::Value>().await?;
    let object = sha.get("object").ok_or("Failed to get object")?;
    let sha = object
        .get("sha")
        .ok_or("Failed to get sha")?
        .as_str()
        .ok_or("Failed to parse SHA as str")?;

    // Create production branch using github api and github_pat
    let res = client
        .post(format!(
            "https://api.github.com/repos/{}/git/refs",
            crate::config::CONFIG.github_repo,
        ))
        .basic_auth(
            &crate::config::CONFIG.github_username,
            Some(&crate::config::CONFIG.github_pat),
        )
        .header("User-Agent", &crate::config::CONFIG.github_username)
        .json(&serde_json::json!({
            "ref": "refs/heads/production",
            "sha": sha
        }))
        .send()
        .await?;

    if res.status() != 201 {
        ctx.say("Failed to create production branch").await?;
        return Ok(());
    }

    // Create branch protection rule to lock writes
    let res = client
        .put(format!(
            "https://api.github.com/repos/{}/branches/production/protection",
            crate::config::CONFIG.github_repo,
        ))
        .basic_auth(
            &crate::config::CONFIG.github_username,
            Some(&crate::config::CONFIG.github_pat),
        )
        .header("User-Agent", &crate::config::CONFIG.github_username)
        .json(&serde_json::json!({
            "required_status_checks": null,
            "enforce_admins": true,
            "required_pull_request_reviews": {
                "dismissal_restrictions": {
                    "users": [],
                    "teams": []
                },
                "dismiss_stale_reviews": true,
                "require_code_owner_reviews": true,
                "required_approving_review_count": 1
            },
            "restrictions": {
                "users": [],
                "teams": [],
                "apps": []
            },
            "allow_deletions": false,
            "block_creations": false,
            "lock_branch": true,
        }))
        .send()
        .await?;

    if res.status() != 200 {
        let body = res.text().await?;
        ctx.say(format!("Failed to create production branch protection rule: {}", body)).await?;
        return Ok(());
    }

    // Admin enforce
    let res = client
        .post(format!(
            "https://api.github.com/repos/{}/branches/production/protection/enforce_admins",
            crate::config::CONFIG.github_repo,
        ))
        .basic_auth(
            &crate::config::CONFIG.github_username,
            Some(&crate::config::CONFIG.github_pat),
        )
        .header("User-Agent", &crate::config::CONFIG.github_username)
        .send()
        .await?;

    if res.status() != 200 {
        let body = res.text().await?;
        ctx.say(format!("Failed to enforce production branch protection rule: {}", body)).await?;
        return Ok(());
    }    

    ctx.say("Done!").await?;

    Ok(())
}

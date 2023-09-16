use log::{error, info, warn};
use serenity::{
    all::ChannelId,
    builder::{CreateEmbed, CreateEmbedFooter, CreateMessage},
};

use crate::impls::target_types::TargetType;

pub async fn deleted_bots(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    let bot_ids = sqlx::query!("SELECT bot_id FROM bots")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error while fetching all bots: {}", e))?;

    for bot in bot_ids {
        // Fetch bot from dovewing
        let bot_id = bot.bot_id;

        let Ok(res) = sqlx::query!(
            "SELECT username FROM internal_user_cache__discord WHERE id = $1",
            bot_id
        )
        .fetch_one(pool)
        .await
        else {
            warn!(
                "Bot {} is not in internal_user_cache__discord, forcing indexing of bot",
                bot_id
            );

            let Ok(req) = reqwest::get(format!(
                "{}/platform/user/{}?platform=discord",
                crate::config::CONFIG.popplio_url,
                bot_id
            ))
            .await
            else {
                error!("Failed to fetch bot {} from Popplio", bot_id);
                continue;
            };

            if !req.status().is_success() {
                error!("Failed to fetch bot {} from Popplio", bot_id);
                continue;
            }

            continue;
        };

        if res.username.starts_with("Deleted User") {
            info!(
                "Bot {} is potentially deleted, checking with Discord API",
                bot_id
            );

            // Bot may be deleted, check using RPC endpoint
            let req = reqwest::get(format!(
                "{}/api/v10/applications/{}/rpc",
                crate::config::CONFIG.proxy_url,
                bot_id
            ))
            .await
            .map_err(|e| {
                format!(
                    "Error while fetching RPC endpoint for bot {}: {}",
                    bot_id, e
                )
            })?;

            if req.status().is_success() {
                // Bot is not deleted
                continue;
            }

            info!(
                "Bot {} is deleted from Discord, removing from database",
                bot_id
            );

            // Bot is deleted, remove from database
            let owners =
                crate::impls::utils::get_entity_managers(TargetType::Bot, &bot_id, pool).await?;

            let mut tx = pool
                .begin()
                .await
                .map_err(|e| format!("Error creating transaction: {}", e))?;

            sqlx::query!("DELETE FROM bots WHERE bot_id = $1", bot_id)
                .execute(&mut tx)
                .await
                .map_err(|e| format!("Error while deleting bot {} from database: {}", bot_id, e))?;

            // Send message to mod logs channel
            let msg = CreateMessage::default()
                .content(owners.mention_users())
                .embed(
                    CreateEmbed::default()
                        .title("Bot Deleted From Discord!")
                        .url(format!(
                            "{}/bots/{}",
                            crate::config::CONFIG.frontend_url,
                            bot_id
                        ))
                        .description(format!(
                            "`{}` has been deleted from Discord, and so will be removed from list!",
                            bot_id
                        ))
                        .field("Bot", bot_id, true)
                        .footer(CreateEmbedFooter::new(
                            "If this is a mistake, please contact support!",
                        ))
                        .color(0x00ff00),
                );

            ChannelId(crate::config::CONFIG.channels.mod_logs)
                .send_message(&cache_http, msg)
                .await?;

            tx.commit().await?;
        }
    }

    Ok(())
}

use std::num::NonZeroU64;

use poise::serenity_prelude::{GuildId, UserId, CreateEmbed, CreateEmbedFooter, CreateMessage, RoleId, Mentionable, ChannelId};

pub async fn uptime_checker(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    let subject_rows = sqlx::query!(
        "SELECT bot_id, uptime, total_uptime FROM bots WHERE type = 'approved' OR type = 'certified'"
    )
    .fetch_all(pool)
    .await?;

    let presences = {
        if let Some(guild) = cache_http.cache.guild(GuildId(crate::config::CONFIG.servers.main)) {
            Some(guild.presences.clone())
        } else {
            None
        }
    }
    .ok_or("Could not find main server")?;

    for row in subject_rows {
        // Find bot in cache
        let bot_snow = match row.bot_id.parse::<NonZeroU64>() {
            Ok(snow) => snow,
            Err(_) => {
                log::warn!("Invalid bot id: {}", row.bot_id);
                continue;
            }
        };

        // Find user in precense cache
        match presences.get(&UserId(bot_snow)) {
            Some(precense) => {
                let uptime = precense.status != poise::serenity_prelude::OnlineStatus::Offline;

                if uptime {
                    sqlx::query!(
                        "UPDATE bots SET uptime = uptime + 1, total_uptime = total_uptime + 1 WHERE bot_id = $1",
                        row.bot_id
                    )
                    .execute(pool)
                    .await?;
                } else {
                    log::warn!("Bot {} is offline", row.bot_id);
                    sqlx::query!(
                        "UPDATE bots SET total_uptime = total_uptime + 1 WHERE bot_id = $1",
                        row.bot_id
                    )
                    .execute(pool)
                    .await?;

                    let uptime_rate = ((row.uptime + 1) / row.total_uptime) * 100;

                    if uptime_rate < 50 && row.total_uptime > 20 {
                        // Send message to mod logs
                        let ping = crate::impls::utils::resolve_ping_user(&row.bot_id, pool).await?;

                        let msg = CreateMessage::default()
                        .content(format!("<@!{}> {}", ping, RoleId(crate::config::CONFIG.roles.web_moderator).mention()))
                        .embed(
                            CreateEmbed::default()
                                .title("Bot Uptime Warning!")
                                .url(format!("{}/bots/{}", crate::config::CONFIG.frontend_url, row.bot_id))
                                .description(format!("<@!{}> a lower uptime than 50% with over 20 uptime checks", row.bot_id))
                                .field("Bot", "<@!".to_string() + &row.bot_id + ">", true)
                                .footer(CreateEmbedFooter::new("Please check this bot and ensure its actually alive!"))
                                .color(0x00ff00),
                        );  

                        ChannelId(crate::config::CONFIG.channels.mod_logs)
                        .send_message(&cache_http, msg)
                        .await?;                              
                    }
                }
            },
            None => {
                log::warn!("Bot {} is not in cache, possibly not on main server?", row.bot_id);
                continue;
            }
        }
    }

    Ok(())
}
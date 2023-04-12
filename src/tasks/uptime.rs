use std::num::NonZeroU64;

use log::info;
use poise::serenity_prelude::{GuildId, CreateEmbed, CreateEmbedFooter, CreateMessage, ChannelId};

pub async fn uptime_checker(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    let subject_rows = sqlx::query!(
        "SELECT bot_id, uptime, total_uptime FROM bots WHERE (type = 'approved' OR type = 'certified') AND (NOW() - uptime_last_checked > interval '30 minutes')"
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
        match cache_http.cache.member_field(GuildId(crate::config::CONFIG.servers.main), bot_snow, |m| m.user.id) {
            Some(precense) => {
                let uptime = match presences.get(&precense) {
                    Some(precense) => {
                        precense.status != poise::serenity_prelude::OnlineStatus::Offline
                    },
                    None => {
                        false
                    }
                };

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

                    let uptime_rate = ((row.uptime + 1) / (row.total_uptime + 1)) * 100;

                    info!("Uptime rate: {} for bot {}", uptime_rate, row.bot_id);

                    if uptime_rate < 50 && row.total_uptime > 25 {
                        // Send message to mod logs
                        let msg = CreateMessage::default()
                        .embed(
                            CreateEmbed::default()
                                .title("Bot Uptime Warning!")
                                .url(format!("{}/bots/{}", crate::config::CONFIG.frontend_url, row.bot_id))
                                .description(format!("<@!{}> a lower uptime than 50% with over 25 uptime checks", row.bot_id))
                                .field("Bot", "<@!".to_string() + &row.bot_id + ">", true)
                                .footer(CreateEmbedFooter::new("Please check this bot and ensure its actually alive!"))
                                .color(0x00ff00),
                        );  

                        ChannelId(crate::config::CONFIG.channels.uptime)
                        .send_message(&cache_http, msg)
                        .await?;                              
                    }

                    sqlx::query!(
                        "UPDATE bots SET uptime_last_checked = NOW() WHERE bot_id = $1",
                        row.bot_id
                    )
                    .execute(pool)
                    .await?;
                }
            },
            None => {
                log::warn!("Could not find bot {} in cache", row.bot_id);
                continue;
            }
        }
    }

    Ok(())
}

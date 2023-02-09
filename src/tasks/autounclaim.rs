use poise::serenity_prelude::{ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, UserId};
use std::num::NonZeroU64;

use crate::config;

pub async fn auto_unclaim(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    let bots = sqlx::query!(
        "SELECT bot_id, claimed_by, last_claimed, owner FROM bots WHERE type = 'claimed' AND NOW() - last_claimed > INTERVAL '1 hour'",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error while checking for claimed bots: {}", e))?;

    for bot in bots {
        if bot.claimed_by.is_none() {
            log::info!(
                "Unclaiming bot {} because it has no staff who has claimed it",
                bot.bot_id
            );
            sqlx::query!(
                "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
                bot.bot_id
            )
            .execute(pool)
            .await
            .map_err(|e| format!("Error while unclaiming bot {}: {}", bot.bot_id, e))?;

            continue;
        }

        if bot.last_claimed.is_none() {
            log::info!(
                "Unclaiming bot {} because it has no staff who has claimed it",
                bot.bot_id
            );
            sqlx::query!(
                "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
                bot.bot_id
            )
            .execute(pool)
            .await
            .map_err(|e| format!("Error while unclaiming bot {}: {}", bot.bot_id, e))?;

            continue;
        }

        if let Some(claimed_by) = bot.claimed_by {
            if let Some(last_claimed) = bot.last_claimed {
                log::info!(
                    "Unclaiming bot {} because it was claimed by {} and never unclaimed",
                    bot.bot_id,
                    claimed_by
                );   
                
                sqlx::query!(
                    "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
                    bot.bot_id
                )
                .execute(pool)
                .await
                .map_err(|e| format!("Error while unclaiming bot {}: {}", bot.bot_id, e))?;  
                
                // Now send message in #lounge
                let msg = CreateMessage::default()
                .content(format!("<@{}>", claimed_by))
                .embed(
                    CreateEmbed::default()
                        .title("Auto-Unclaimed Bot")
                        .description(
                            format!(
                                "Bot <@{}> was auto-unclaimed (was previously claimed by <@{}> due to it being claimed for over one hour without being approved or denied).\nThis bot was last claimed <t:{}:R>.", 
                                bot.bot_id,
                                claimed_by,
                                last_claimed.timestamp(),
                            ))
                        .color(0xFF0000)
                );    

                ChannelId(config::CONFIG.channels.testing_lounge)
                .send_message(&cache_http, msg)
                .await
                .map_err(|e| format!("Error while sending message in #lounge: {}", e))?;

                if let Ok(owner) = bot.owner.parse::<NonZeroU64>() {
                    // Check that owner is in the server

                    if cache_http.cache.member_field(config::CONFIG.servers.main, owner, |m| m.user.id).is_none() {
                        log::warn!("Bot owner is not in the server. Not sending DM");
                        continue;
                    }

                    match UserId(owner).create_dm_channel(&cache_http).await {
                        Ok(dm) => {
                            let msg = CreateMessage::default()
                            .embed(
                                CreateEmbed::default()
                                    .title("Bot Unclaimed!")
                                    .description(
                                        format!(
                                            r#"
<@{}> has been unclaimed as it was not being actively reviewed. 

Don't worry, this is normal, could just be our staff looking more into your bots functionality! 

For more information, you can contact the current reviewer <@{}>

*This bot was claimed <t:{}:R>. This is a automated message letting you know about whats going on...*
                                            "#, 
                                            bot.bot_id,
                                            claimed_by,
                                            last_claimed.timestamp()
                                        ))
                                    .footer(CreateEmbedFooter::new("This is completely normal, don't worry!"))
                            );
        
                            dm.send_message(&cache_http, msg)
                            .await
                            .map_err(|e| format!("Error while sending message in DM: {}", e))?;        
                        },
                        Err(e) => {
                            log::warn!("Error while creating DM channel with bot owner: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
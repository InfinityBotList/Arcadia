use poise::serenity_prelude::{ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, UserId};
use std::{num::NonZeroU64, time::Duration};

use crate::config;

pub async fn autounclaim_task(
    pool: sqlx::PgPool,
    cache_http: crate::impls::cache::CacheHttpImpl,
) -> ! {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        log::info!("TASK: autounclaim_task (60s interval) [Checking for claimed bots greater than 1 hour claim interval]");

        let res = sqlx::query!(
            "SELECT bot_id, claimed_by, last_claimed, owner FROM bots WHERE type = 'claimed' AND NOW() - last_claimed > INTERVAL '1 hour'",
        )
        .fetch_all(&pool)
        .await;

        if res.is_err() {
            log::error!(
                "Error while checking for claimed bots: {:?}",
                res.unwrap_err()
            );
            continue;
        }

        let bots = res.unwrap();

        for bot in bots {
            if bot.claimed_by.is_none() {
                log::info!(
                    "Unclaiming bot {} because it has no staff who has claimed it",
                    bot.bot_id
                );
                let res = sqlx::query!(
                    "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
                    bot.bot_id
                )
                .execute(&pool)
                .await;

                if res.is_err() {
                    log::error!(
                        "Error while unclaiming bot {}: {:?}",
                        bot.bot_id,
                        res.unwrap_err()
                    );
                    continue;
                }

                continue;
            }

            if bot.last_claimed.is_none() {
                log::info!(
                    "Unclaiming bot {} because it has no last_claimed time",
                    bot.bot_id
                );
                let res = sqlx::query!(
                    "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
                    bot.bot_id
                )
                .execute(&pool)
                .await;

                if res.is_err() {
                    log::error!(
                        "Error while unclaiming bot {}: {:?}",
                        bot.bot_id,
                        res.unwrap_err()
                    );
                    continue;
                }

                continue;
            }

            let claimed_by = bot.claimed_by.unwrap();
            let last_claimed = bot.last_claimed.unwrap();

            log::info!(
                "Unclaiming bot {} because it was claimed by {} and never unclaimed",
                bot.bot_id,
                claimed_by
            );
            let res = sqlx::query!(
                "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
                bot.bot_id
            )
            .execute(&pool)
            .await;

            if res.is_err() {
                log::error!(
                    "Error while unclaiming bot {}: {:?}",
                    bot.bot_id,
                    res.unwrap_err()
                );
                continue;
            }

            let start_time = chrono::offset::Utc::now();

            // Now send message in #lounge
            let msg = CreateMessage::default()
                .content(format!("<@{}>", claimed_by))
                .embed(
                    CreateEmbed::default()
                        .title("Auto-Unclaimed Bot")
                        .description(
                            format!(
                                "Bot <@{}> was auto-unclaimed (was previously claimed by <@{}> due to it being claimed for over one hour without being approved or denied).\nThis bot was last claimed at {} ({}).", 
                                bot.bot_id,
                                claimed_by,
                                last_claimed.format("%Y-%m-%d %H:%M:%S"),
                                (start_time - last_claimed).num_minutes().to_string() + " minutes ago"
                            ))
                        .color(0xFF0000)
                );

            let err = ChannelId(config::CONFIG.channels.testing_lounge)
                .send_message(&cache_http, msg)
                .await;

            if err.is_err() {
                log::error!(
                    "Error while sending message to lounge: {:?}",
                    err.unwrap_err()
                );
                continue;
            }

            let owner = bot.owner.parse::<NonZeroU64>();

            if let Ok(owner) = owner {
                let private_channel = UserId(owner).create_dm_channel(&cache_http).await;

                if private_channel.is_err() {
                    log::error!(
                        "Error while sending message to owner: {:?}",
                        private_channel.unwrap_err()
                    );
                    continue;
                }

                let private_channel = private_channel.unwrap();

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

*This bot was claimed at {} ({}). This is a automated message letting you know about whats going on...*
                                    "#, 
                                    bot.bot_id,
                                    claimed_by,
                                    last_claimed.format("%Y-%m-%d %H:%M:%S"),
                                    (start_time - last_claimed).num_minutes().to_string() + " minutes ago"
                                ))
                            .footer(CreateEmbedFooter::new("This is completely normal, don't worry!"))
                    );

                let err = private_channel.send_message(&cache_http, msg).await;

                if err.is_err() {
                    log::error!(
                        "Error while sending message to owner: {:?}",
                        err.unwrap_err()
                    );
                    continue;
                }
            }
        }
    }
}

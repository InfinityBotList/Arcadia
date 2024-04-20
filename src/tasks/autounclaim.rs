use poise::serenity_prelude::{CreateEmbed, CreateEmbedFooter, CreateMessage};

use crate::{config, impls::target_types::TargetType};

// Internal struct used to send notifications on unclaimed bots
struct AutoUnclaimNotification {
    bot_id: String,
    claimed_by: String,
    last_claimed: chrono::DateTime<chrono::Utc>,
}

pub async fn auto_unclaim(ctx: &serenity::all::Context) -> Result<(), crate::Error> {
    let data = ctx.data::<crate::Data>();
    let pool = &data.pool;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Error creating transaction: {:?}", e))?;

    let mut notifications = Vec::new();

    let bots = sqlx::query!(
        "SELECT bot_id, claimed_by, last_claimed FROM bots WHERE claimed_by IS NOT NULL AND NOW() - last_claimed > INTERVAL '1 hour' FOR UPDATE",
    )
    .fetch_all(&mut *tx)
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
            .execute(&mut *tx)
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
            .execute(&mut *tx)
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
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Error while unclaiming bot {}: {}", bot.bot_id, e))?;

                notifications.push(AutoUnclaimNotification {
                    bot_id: bot.bot_id,
                    claimed_by,
                    last_claimed,
                });
            }
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Error while committing transaction: {}", e))?;

    for notification in notifications {
        // Now send message in #lounge
        let msg = CreateMessage::default()
        .content(format!("<@{}>", notification.claimed_by))
        .embed(
            CreateEmbed::default()
                .title("Auto-Unclaimed Bot")
                .description(
                    format!(
                        "Bot <@{}> was auto-unclaimed (was previously claimed by <@{}> due to it being claimed for over one hour without being approved or denied).\nThis bot was last claimed <t:{}:R>.", 
                        notification.bot_id,
                        notification.claimed_by,
                        notification.last_claimed.timestamp(),
                    ))
                .color(0xFF0000)
        );

        config::CONFIG
            .channels
            .testing_lounge
            .send_message(ctx, msg)
            .await
            .map_err(|e| format!("Error while sending message in #lounge: {}", e))?;

        let owners =
            crate::impls::utils::get_entity_managers(TargetType::Bot, &notification.bot_id, pool)
                .await?;

        config::CONFIG.channels.mod_logs
        .send_message(
            ctx,
            CreateMessage::default()
            .content(owners.mention_users())
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
                            notification.bot_id,
                            notification.claimed_by,
                            notification.last_claimed.timestamp()
                        ))
                    .footer(CreateEmbedFooter::new("This is completely normal, don't worry!"))
                )
            )
            .await
            .map_err(|e| format!("Error while sending message in #mod-logs: {}", e))?;
    }

    Ok(())
}

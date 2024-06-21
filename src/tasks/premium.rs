use crate::impls::target_types::TargetType;
use poise::serenity_prelude::{CreateMessage, UserId};
use serenity::all::Mentionable;

struct BotData {
    bot_id: UserId,
    bot_username: String,
    bot_type: String,
}

pub async fn premium_remove(ctx: &serenity::client::Context) -> Result<(), crate::Error> {
    let data = ctx.data::<crate::Data>();
    let pool = &data.pool;

    let res = sqlx::query!(
        "
        SELECT bot_id, start_premium_period, premium_period_length, type FROM bots 
		WHERE (
			premium = true
			AND (
				(type != 'approved' AND type != 'certified')
				OR (start_premium_period + premium_period_length) < NOW()
			)
		)
        "
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error while checking for expired premium bots: {}", e))?;

    let mut bot_data = vec![];
    for row in res {
        let bot_id = row
            .bot_id
            .parse::<UserId>()
            .map_err(|e| format!("Error while parsing bot id: {}", e))?;

        let bot_partial = crate::impls::dovewing::get_platform_user(
            pool,
            crate::impls::dovewing::DovewingSource::Discord(botox::cache::CacheHttpImpl::from_ctx(
                ctx,
            )),
            &bot_id.to_string(),
        )
        .await?;

        bot_data.push(BotData {
            bot_id,
            bot_username: bot_partial.username.to_string(),
            bot_type: row.r#type,
        });
    }

    for bot in bot_data {
        log::info!("Removing premium from bot {}", bot.bot_id);

        sqlx::query!(
            "UPDATE bots SET premium = false WHERE bot_id = $1",
            bot.bot_id.to_string()
        )
        .execute(pool)
        .await
        .map_err(|e| {
            format!(
                "Error while removing premium from bot {}: {}",
                bot.bot_id, e
            )
        })?;

        let owners = crate::impls::utils::get_entity_managers(
            TargetType::Bot,
            &bot.bot_id.to_string(),
            pool,
        )
        .await?;

        let msg = {
            if bot.bot_type != "approved" && bot.bot_type != "certified" {
                format!(
                    "{} ({}) by {} has been removed from the premium list because it is not/no longer approved or certified.", 
                    bot.bot_id.mention(),
                    bot.bot_username,
                    owners.mention_users(),
                )
            } else {
                format!(
                    "{} ({}) by {} has been removed from the premium list as their subscription has expired.", 
                    bot.bot_id.mention(),
                    bot.bot_username,
                    owners.mention_users(),
                )
            }
        };

        crate::config::CONFIG
            .channels
            .mod_logs
            .send_message(&ctx.http, CreateMessage::new().content(msg))
            .await?;
    }

    Ok(())
}

use poise::serenity_prelude::{ChannelId, CreateMessage};

pub async fn premium_remove(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
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

    for row in res {
        log::info!("Removing premium from bot {}", row.bot_id);

        sqlx::query!(
            "UPDATE bots SET premium = false WHERE bot_id = $1",
            row.bot_id
        )
        .execute(pool)
        .await
        .map_err(|e| {
            format!(
                "Error while removing premium from bot {}: {}",
                row.bot_id, e
            )
        })?;

        let bot_id = row
            .bot_id
            .parse()
            .map_err(|e| format!("Error while parsing bot id: {}", e))?;

        let bot_username = {
            if let Some(name) =
                cache_http
                    .cache
                    .member_field(crate::config::CONFIG.servers.main, bot_id, |m| {
                        m.user.name.clone()
                    })
            {
                name
            } else {
                // Get bot from API
                {
                    let bot = cache_http.http.get_user(bot_id).await?;

                    bot.name
                }
            }
        };

        let bot_owner = crate::impls::utils::resolve_ping_user(&bot_id.to_string(), pool).await?;

        let msg = {
            if row.r#type != "approved" && row.r#type != "certified" {
                format!(
                    "<@{}> ({}) by <@{}> has been removed from the premium list because it is not/no longer approved or certified [v4].", 
                    bot_id, 
                    bot_username,
                    bot_owner,
                )
            } else {
                format!(
                    "<@{}> ({}) by <@{}> has been removed from the premium list as their subscription has expired [v4].", 
                    bot_id, 
                    bot_username,
                    bot_owner,
                )
            }
        };

        ChannelId(crate::config::CONFIG.channels.mod_logs)
            .send_message(&cache_http, CreateMessage::default().content(msg))
            .await?;
    }

    Ok(())
}

use crate::config;

pub async fn dead_guilds(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    let current_user = cache_http.cache.current_user().id.0;

    for guild_id in cache_http.cache.guilds() { 
        if vec![
            config::CONFIG.servers.main, 
            config::CONFIG.servers.staff, 
            config::CONFIG.servers.testing
        ].contains(&guild_id.0) {
            continue;
        }

        let guild_owner = {
            let guild = cache_http.cache.guild(guild_id);

            if let Some(guild) = guild {                
                guild.owner_id
            } else {
                continue
            }
        };

        let res = sqlx::query!(
            "SELECT COUNT(*) FROM users WHERE staff_onboard_guild = $1 AND NOW() - staff_onboard_last_start_time < interval '1 hour' AND NOT(staff_onboard_state = $2 OR staff_onboard_state = $3)",
            guild_id.0.to_string(),
            crate::onboarding::OnboardState::Completed.as_str(),
            crate::onboarding::OnboardState::PendingManagerReview.as_str()
        )
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Error while checking for dead guilds: {}", e))?;

        if res.count.unwrap_or_default() == 0 {
            // This guild can be deleted or left
            if guild_owner.0 == current_user {
                if let Err(e) = guild_id.delete(&cache_http.http).await {
                    log::error!(
                        "Error while deleting guild {}: {:?}",
                        guild_id.0,
                        e
                    );
                }
            } else if let Err(e) = guild_id.leave(&cache_http.http).await {
                log::error!(
                    "Error while leaving guild {}: {:?}",
                    guild_id.0,
                    e
                );
            }
        }
    }

    Ok(())
}
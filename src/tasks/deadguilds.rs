use std::time::Duration;

use crate::config;

pub async fn deadguilds_task(
    pool: sqlx::PgPool,
    cache_http: crate::impls::cache::CacheHttpImpl,
) -> ! {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        log::info!("TASK: deadguilds_task (60s interval) [Checking for dead guilds made by staff or bad users]");

        // Loop through all guilds
        let guilds = cache_http.cache.guilds();

        let http = cache_http.http.clone();

        let bowner = cache_http.cache.current_user().id.0;

        log::info!("Checking {} guilds", guilds.len());

        // We do this to avoid the async cache guard introduced in serenity next
        for guild_id in guilds {
            let guild_owner = cache_http.cache.guild(guild_id).unwrap().owner_id;
            // Check if guild is official (main/testing/staff)
            if guild_id.0 == config::CONFIG.servers.main
                || guild_id.0 == config::CONFIG.servers.staff
                || guild_id.0 == config::CONFIG.servers.testing
            {
                continue;
            }

            let res = sqlx::query!(
                "SELECT COUNT(*) FROM users WHERE staff_onboard_guild = $1 AND NOW() - staff_onboard_last_start_time < interval '1 hour' AND NOT(staff_onboard_state = $2 OR staff_onboard_state = $3)",
                guild_id.0.to_string(),
                crate::onboarding::OnboardState::Completed.as_str(),
                crate::onboarding::OnboardState::PendingManagerReview.as_str()
            )
            .fetch_one(&pool)
            .await;

            if res.is_err() {
                log::error!(
                    "Error while checking for staff onboarding guild {}: {:?}",
                    guild_id.0,
                    res.unwrap_err()
                );
                continue;
            }

            let res = res.unwrap();

            if res.count.unwrap_or_default() == 0 {
                // This guild can be deleted or left
                if guild_owner.0 == bowner {
                    let err = guild_id.delete(&http).await;

                    if err.is_err() {
                        log::error!(
                            "Error while deleting guild {}: {:?}",
                            guild_id.0,
                            err.unwrap_err()
                        );
                    }
                } else {
                    let err = guild_id.leave(&http).await;

                    if err.is_err() {
                        log::error!(
                            "Error while leaving guild {}: {:?}",
                            guild_id.0,
                            err.unwrap_err()
                        );
                    }
                }
            }
        }
    }
}

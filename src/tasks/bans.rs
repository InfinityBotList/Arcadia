use std::time::Duration;

use poise::serenity_prelude::GuildId;

use crate::config;

pub async fn bans_sync_task(
    pool: sqlx::PgPool,
    cache_http: crate::impls::cache::CacheHttpImpl,
) -> ! {
    let mut interval = tokio::time::interval(Duration::from_secs(300));

    loop {
        interval.tick().await;

        log::info!("TASK: bans_sync_task (300s interval)");

        let bans = GuildId(config::CONFIG.servers.main)
            .bans(&cache_http.http)
            .await;

        if let Err(e) = bans {
            log::error!("Error while fetching bans: {}", e);
            continue;
        }

        let bans = bans.unwrap();

        // Create a transaction
        let tx = pool.begin().await;

        if let Err(e) = tx {
            log::error!("Error creating transaction: {}", e);
            continue;
        }

        let mut tx = tx.unwrap();

        // First unset all bans
        if let Err(e) = sqlx::query!("UPDATE users SET banned = false")
            .execute(&mut tx)
            .await
        {
            log::error!("Error while updating users in database: {}", e);
            continue;
        }

        for ban in bans {
            let user_id = ban.user.id.0.to_string();
            let res = sqlx::query!("UPDATE users SET banned = true WHERE user_id = $1", user_id)
                .execute(&mut tx)
                .await;

            if res.is_err() {
                log::error!(
                    "Error while updating user {} in database: {:?}",
                    user_id,
                    res.unwrap_err()
                );
                continue;
            }
        }

        // Commit the transaction
        if let Err(e) = tx.commit().await {
            log::error!("Error while committing transaction: {}", e);
            continue;
        }
    }
}

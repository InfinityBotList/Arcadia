use poise::serenity_prelude::GuildId;

use crate::config;

pub async fn bans_sync(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    let bans = GuildId(config::CONFIG.servers.main)
    .bans(&cache_http.http)
    .await
    .map_err(|e| format!("Error while fetching bans: {}", e))?;

    // Create a transaction
    let mut tx = pool.begin().await.map_err(|e| format!("Error creating transaction: {}", e))?;

    // First unset all bans
    sqlx::query!("UPDATE users SET banned = false")
        .execute(&mut tx)
        .await
        .map_err(|e| format!("Error while updating users in database: {}", e))?;

    for ban in bans {
        let user_id = ban.user.id.0.to_string();
        sqlx::query!("UPDATE users SET banned = true WHERE user_id = $1", user_id)
            .execute(&mut tx)
            .await
            .map_err(|e| format!("Error while updating user {} in database: {:?}", user_id, e))?;
    }

    // Commit the transaction
    tx.commit().await.map_err(|e| format!("Error while committing transaction: {}", e))?;

    Ok(())
}
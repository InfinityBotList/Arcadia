use poise::serenity_prelude::UserId;

use crate::config;

pub enum SpecialRole {
    BugHunter,
}

struct SpecRoleSync {
    user_id: UserId,
    col: SpecialRole,
}

pub async fn spec_role_sync(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    // Now actually resync
    let mut resync = Vec::new();

    {
        if let Some(guild) = cache_http.cache.guild(config::CONFIG.servers.main) {
            // Get all members with the bug hunter role
            for member in guild.members.iter() {
                if member.roles.contains(&config::CONFIG.roles.bug_hunters) {
                    resync.push(SpecRoleSync {
                        user_id: member.user.id,
                        col: SpecialRole::BugHunter,
                    });
                }
            }
        } else {
            return Err("Failed to get guild".into());
        }
    }

    // Create a transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Error creating transaction: {:?}", e))?;

    sqlx::query!("UPDATE users SET bug_hunters = false")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error updating users: {:?}", e))?;

    for user in resync {
        match user.col {
            SpecialRole::BugHunter => {
                sqlx::query!(
                    "
                    UPDATE users SET bug_hunters = true WHERE user_id = $1",
                    user.user_id.to_string()
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Error updating users: {:?}", e))?;
            }
        }
    }

    tx.commit().await?;

    Ok(())
}

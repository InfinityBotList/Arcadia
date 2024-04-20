use std::collections::HashSet;

use serenity::all::Color;
use serenity::builder::{CreateEmbed, CreateMessage};

use crate::config;

pub async fn bans_sync(ctx: &serenity::all::Context) -> Result<(), crate::Error> {
    let data = ctx.data::<crate::Data>();
    let pool = &data.pool;

    let bans = config::CONFIG
        .servers
        .main
        .bans(&ctx.http, None, None)
        .await
        .map_err(|e| format!("Error while fetching bans: {}", e))?;

    let db_records = sqlx::query!("SELECT user_id FROM users WHERE banned = true")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error while fetching bans from database: {}", e))?;

    let mut ping_users = "".to_string();
    for user in &crate::config::CONFIG.owners {
        ping_users.push_str(&format!("<@{}>", user));
    }

    // Next find the symmetric difference between bans and db_bans
    //
    // If a member is in bans but not in db_bans, they should be banned
    // If a member is in db_bans but not in bans, they should be unbanned
    let mut server_bans = HashSet::new();
    let mut db_bans = HashSet::new();
    let mut user_banned_map = Vec::new();

    for ban in bans {
        server_bans.insert(ban.user.id.to_string());
        user_banned_map.push(ban.user.id.to_string());
    }

    for ban in db_records {
        db_bans.insert(ban.user_id);
    }

    let to_modify = server_bans.symmetric_difference(&db_bans);

    log::warn!("Bans to modify: {:?}", &to_modify);

    for user_id in to_modify {
        let is_banned = user_banned_map.contains(user_id);
        let res = sqlx::query!(
            "UPDATE users SET banned = $1 WHERE user_id = $2",
            is_banned,
            user_id
        )
        .execute(pool)
        .await
        .map_err(|e| format!("Error while updating user {} in database: {:?}", user_id, e))?;

        if res.rows_affected() == 0 {
            sqlx::query!(
                "INSERT INTO users (user_id, banned, api_token) VALUES ($1, $2, $3)",
                user_id,
                is_banned,
                botox::crypto::gen_random(512)
            )
            .execute(pool)
            .await
            .map_err(|e| {
                format!(
                    "Error while inserting user {} into database: {:?}",
                    user_id, e
                )
            })?;
        }

        if is_banned {
            crate::config::CONFIG
                .channels
                .mod_logs
                .send_message(
                    &ctx,
                    CreateMessage::new()
                        .content(&ping_users)
                        .embeds(vec![CreateEmbed::new()
                            .title("User Ban")
                            .description(format!("User {} was banned", user_id))
                            .color(Color::RED)]),
                )
                .await?;
        } else {
            crate::config::CONFIG
                .channels
                .mod_logs
                .send_message(
                    &ctx,
                    CreateMessage::new()
                        .content(&ping_users)
                        .embeds(vec![CreateEmbed::new()
                            .title("User Unban")
                            .description(format!("User {} was unbanned", user_id))
                            .color(Color::BLURPLE)]),
                )
                .await?;
        }
    }

    Ok(())
}

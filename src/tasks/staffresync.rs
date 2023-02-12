use std::num::NonZeroU64;

use crate::config;

enum StaffPosition {
    Staff,
    Manager,
    HeadManager,
    Developer,
    HeadDeveloper,
}

struct StaffResync {
    user_id: NonZeroU64,
    col: StaffPosition,
}

pub async fn staff_resync(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    // Remove bad users
    sqlx::query!("UPDATE users SET user_id = TRIM(user_id)")
        .execute(pool)
        .await
        .map_err(|e| format!("Error while trimming user_id: {:?}", e))?;

    sqlx::query!("DELETE FROM users WHERE user_id = ''")
        .execute(pool)
        .await
        .map_err(|e| format!("Error while removing empty user_id: {:?}", e))?;

    // Now actually resync
    let mut staff_resync = Vec::new();

    let dev_role = poise::serenity_prelude::RoleId(config::CONFIG.roles.developer);
    let head_dev_role = poise::serenity_prelude::RoleId(config::CONFIG.roles.head_developer);
    let staff_man_role = poise::serenity_prelude::RoleId(config::CONFIG.roles.staff_manager);
    let head_man_role = poise::serenity_prelude::RoleId(config::CONFIG.roles.head_manager);
    let web_mod_role = poise::serenity_prelude::RoleId(config::CONFIG.roles.web_moderator);

    {
        if let Some(guild) = cache_http.cache.guild(config::CONFIG.servers.staff) {
            for (_, member) in guild.members.iter() {
                if member.roles.contains(&dev_role) {
                    staff_resync.push(StaffResync {
                        user_id: member.user.id.0,
                        col: StaffPosition::Developer,
                    });
                }
                if member.roles.contains(&head_dev_role) {
                    staff_resync.push(StaffResync {
                        user_id: member.user.id.0,
                        col: StaffPosition::HeadDeveloper,
                    });
                }
                if member.roles.contains(&staff_man_role) {
                    staff_resync.push(StaffResync {
                        user_id: member.user.id.0,
                        col: StaffPosition::Manager,
                    });
                }
                if member.roles.contains(&head_man_role) {
                    staff_resync.push(StaffResync {
                        user_id: member.user.id.0,
                        col: StaffPosition::HeadManager,
                    });
                }
                if member.roles.contains(&web_mod_role) {
                    staff_resync.push(StaffResync {
                        user_id: member.user.id.0,
                        col: StaffPosition::Staff,
                    });
                }
            }
        } else {
            log::warn!("Failed to get guild");
        }
    }

    // Create a transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Error creating transaction: {:?}", e))?;

    // First unset all staff
    sqlx::query!("UPDATE users SET staff = false, ibldev = false, iblhdev = false, admin = false, hadmin = false")
    .execute(&mut tx)
    .await
    .map_err(|e| format!("Error while updating users in database: {:?}", e))?;

    // Now set all staff as per the staff_resync vector
    for staff in staff_resync {
        match staff.col {
            StaffPosition::Staff => {
                sqlx::query!(
                    "UPDATE users SET staff = true WHERE user_id = $1",
                    staff.user_id.to_string()
                )
                .execute(&mut tx)
                .await
            }
            StaffPosition::Manager => {
                sqlx::query!(
                    "UPDATE users SET staff = true, admin = true WHERE user_id = $1",
                    staff.user_id.to_string()
                )
                .execute(&mut tx)
                .await
            }
            StaffPosition::Developer => {
                sqlx::query!(
                    "UPDATE users SET staff = true, ibldev = true WHERE user_id = $1",
                    staff.user_id.to_string()
                )
                .execute(&mut tx)
                .await
            }
            StaffPosition::HeadDeveloper => {
                sqlx::query!(
                "UPDATE users SET staff = true, admin = true, ibldev = true, iblhdev = true WHERE user_id = $1",
                staff.user_id.to_string()
            )
                .execute(&mut tx)
                .await
            }
            StaffPosition::HeadManager => {
                sqlx::query!(
                    "UPDATE users SET staff = true, admin = true, hadmin = true WHERE user_id = $1",
                    staff.user_id.to_string()
                )
                .execute(&mut tx)
                .await
            }
        }
        .map_err(|e| format!("Error while updating users in database: {:?}", e))?;
    }

    // Commit the transaction
    tx.commit()
        .await
        .map_err(|e| format!("Error while committing transaction: {:?}", e))?;

    Ok(())
}

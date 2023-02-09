use std::{num::NonZeroU64, time::Duration};

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

pub async fn staff_resync_task(pool: sqlx::PgPool, cache_http: crate::impls::cache::CacheHttpImpl) -> ! {
    let mut interval = tokio::time::interval(Duration::from_secs(45));

    loop {
        interval.tick().await;

        log::info!("TASK: staff_resync_task (45s interval)");

        if let Err(e) = sqlx::query!("UPDATE users SET user_id = TRIM(user_id)")
        .execute(&pool)
        .await {
            log::error!("Error while trimming user_id: {:?}", e);
        }

        // Then, remove bad users
        let v = sqlx::query!("DELETE FROM users WHERE user_id = ''")
            .execute(&pool)
            .await;

        if v.is_err() {
            log::error!("Error while removing empty user_id: {:?}", v);
        } else {
            let v = v.unwrap();
            if v.rows_affected() > 0 {
                log::info!("Removed {} users with empty user_id", v.rows_affected());
            }
        }

        // Now actually resync
        let mut staff_resync = Vec::new();

        let dev_role = poise::serenity_prelude::RoleId(config::CONFIG.roles.developer);
        let head_dev_role =
            poise::serenity_prelude::RoleId(config::CONFIG.roles.head_developer);
        let staff_man_role =
            poise::serenity_prelude::RoleId(config::CONFIG.roles.staff_manager);
        let head_man_role = poise::serenity_prelude::RoleId(config::CONFIG.roles.head_manager);
        let web_mod_role = poise::serenity_prelude::RoleId(config::CONFIG.roles.web_moderator);

        // Get all members on staff server, this is done in a seperate block due to CacheRef
        {
            let guild = cache_http.cache.guild(config::CONFIG.servers.staff);

            if guild.is_none() {
                log::warn!("Guild not yet cached");
                continue;
            }

            let guild = guild.unwrap();

            for (_, member) in guild.members.iter()
            {
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
        }

        // Create a transaction
        let tx = pool.begin().await;

        if let Err(e) = tx {
            log::error!("Error creating transaction: {}", e);
            continue;
        }

        let mut tx = tx.unwrap();

        // First unset all staff
        if let Err(e) = sqlx::query!("UPDATE users SET staff = false, ibldev = false, iblhdev = false, admin = false, hadmin = false")
        .execute(&mut tx)
        .await {
            log::error!("Error unsetting staff: {}", e);
            continue
        }

        // Now set all staff as per the staff_resync vector
        for staff in staff_resync {
            match staff.col {
                StaffPosition::Staff => {
                    if let Err(e) = sqlx::query!(
                        "UPDATE users SET staff = true WHERE user_id = $1",
                        staff.user_id.to_string()
                    )
                    .execute(&mut tx)
                    .await {
                        log::error!("Error setting staff: {}", e);
                    }
                }
                StaffPosition::Manager => {
                    if let Err(e) = sqlx::query!(
                        "UPDATE users SET staff = true, admin = true WHERE user_id = $1",
                        staff.user_id.to_string()
                    )
                    .execute(&mut tx)
                    .await {
                        log::error!("Error setting staff: {}", e);
                    }
                }
                StaffPosition::Developer => {
                    if let Err(e) = sqlx::query!(
                        "UPDATE users SET staff = true, ibldev = true WHERE user_id = $1",
                        staff.user_id.to_string()
                    )
                    .execute(&mut tx)
                    .await {
                        log::error!("Error setting staff: {}", e);
                    }
                }
                StaffPosition::HeadDeveloper => {
                    if let Err(e) = sqlx::query!("UPDATE users SET staff = true, ibldev = true, iblhdev = true WHERE user_id = $1", staff.user_id.to_string())
                    .execute(&mut tx)
                    .await {
                        log::error!("Error setting staff: {}", e);
                    }
                }
                StaffPosition::HeadManager => {
                    if let Err(e) = sqlx::query!("UPDATE users SET staff = true, admin = true, hadmin = true WHERE user_id = $1", staff.user_id.to_string())
                    .execute(&mut tx)
                    .await {
                        log::error!("Error setting staff: {}", e);
                    }
                }
            }
        }

        // Commit the transaction
        if let Err(e) = tx.commit().await {
            log::error!("Error committing transaction: {}", e);
        }
    }
}
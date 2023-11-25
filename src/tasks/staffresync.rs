use serenity::all::{UserId, RoleId};

use crate::config;

#[derive(Clone, Copy)]
enum StaffPosition {
    Staff,
    Manager,
    HeadManager,
    Developer,
    HeadDeveloper,
    Owner,
}

struct StaffResync {
    user_id: UserId,
    col: StaffPosition,
}

pub async fn staff_resync(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    // Now actually resync
    let mut staff_resync = Vec::new();

    let rid_map: indexmap::IndexMap<RoleId, StaffPosition> = indexmap::indexmap! {
        config::CONFIG.roles.developer => StaffPosition::Developer,
        config::CONFIG.roles.head_developer => StaffPosition::HeadDeveloper,
        config::CONFIG.roles.staff_manager => StaffPosition::Manager,
        config::CONFIG.roles.head_manager => StaffPosition::HeadManager,
        config::CONFIG.roles.web_moderator => StaffPosition::Staff,
    };

    {
        if let Some(guild) = cache_http.cache.guild(config::CONFIG.servers.staff) {
            for (_, member) in guild.members.iter() {
                if config::CONFIG.owners.contains(&member.user.id) {
                    staff_resync.push(StaffResync {
                        user_id: member.user.id,
                        col: StaffPosition::Owner,
                    });
                }

                for (role, col) in rid_map.iter() {
                    if member.roles.contains(role) {
                        staff_resync.push(StaffResync {
                            user_id: member.user.id,
                            col: *col,
                        });
                    }
                }
            }
        } else {
            // Do not continue if we can't get the guild
            return Err("Failed to get guild".into());
        }
    }

    // Create a transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Error creating transaction: {:?}", e))?;

    // First unset all staff
    sqlx::query!(
        "
        UPDATE users SET 
            staff = false, 
            ibldev = false, 
            iblhdev = false, 
            admin = false, 
            hadmin = false,
            owner = false
    "
    )
    .execute(&mut *tx)
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
                .execute(&mut *tx)
                .await
            }
            StaffPosition::Manager => {
                sqlx::query!(
                    "UPDATE users SET staff = true, admin = true WHERE user_id = $1",
                    staff.user_id.to_string()
                )
                .execute(&mut *tx)
                .await
            }
            StaffPosition::Developer => {
                sqlx::query!(
                    "UPDATE users SET staff = true, ibldev = true WHERE user_id = $1",
                    staff.user_id.to_string()
                )
                .execute(&mut *tx)
                .await
            }
            StaffPosition::HeadDeveloper => {
                sqlx::query!(
                "UPDATE users SET staff = true, admin = true, ibldev = true, iblhdev = true WHERE user_id = $1",
                staff.user_id.to_string()
            )
                .execute(&mut *tx)
                .await
            }
            StaffPosition::HeadManager => {
                sqlx::query!(
                    "UPDATE users SET staff = true, admin = true, hadmin = true WHERE user_id = $1",
                    staff.user_id.to_string()
                )
                .execute(&mut *tx)
                .await
            }
            StaffPosition::Owner => {
                sqlx::query!(
                    "UPDATE users SET staff = true, owner = true WHERE user_id = $1",
                    staff.user_id.to_string()
                )
                .execute(&mut *tx)
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

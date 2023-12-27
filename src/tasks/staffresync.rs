use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
};

use serenity::{
    all::UserId,
    builder::{CreateEmbed, CreateMessage},
};
use sqlx::types::Uuid;
use kittycat::perms;

use crate::config;

#[derive(Clone)]
struct CachedPosition {
    /// The id of the position
    id: Uuid,
    /// The name of the position
    name: String,
    /// The role id associated with this position on Discord
    role_id: String,
    /// The index of the permission. Lower means higher in the list of hierarchy
    index: i32,
    /// The preset permissions of this position
    perms: Vec<String>,
}

impl Display for CachedPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}] (<@&{}>)", self.id, self.name, self.role_id)
    }
}

#[derive(Clone)]
struct StaffResync {
    /// The user id of the member
    user_id: UserId,
    /// The list of roles the user has
    roles: Vec<String>,
}

pub async fn staff_resync(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
    // Before doing anything else, get the current list of users with their roles from Discord
    let staff_resync = {
        if let Some(guild) = cache_http.cache.guild(config::CONFIG.servers.staff) {
            let mut staff_resync = Vec::new();

            for (_, member) in guild.members.iter() {
                let mut roles = Vec::new();

                for role in member.roles.iter() {
                    roles.push(role.to_string());
                }

                staff_resync.push(StaffResync {
                    user_id: member.user.id,
                    roles,
                });
            }

            staff_resync
        } else {
            // Do not continue if we can't get the guild
            return Err("Failed to get staff guild for staff perms resync".into());
        }
    };

    // Create a transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Error creating transaction: {:?}", e))?;

    // First get list of positions from db
    let positions = sqlx::query!("SELECT id, name, role_id, index, perms FROM staff_positions")
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Error while getting staff positions: {:?}", e))?;

    // To speed up operations, make a map of id/role_id and perms
    let mut pos_cache_by_id = HashMap::new();
    let mut pos_cache_by_role_id = HashMap::new();
    let mut pos_cache_by_name = HashMap::new();

    for pos in positions {
        pos_cache_by_id.insert(
            pos.id,
            CachedPosition {
                id: pos.id,
                name: pos.name.clone(),
                role_id: pos.role_id.clone(),
                index: pos.index,
                perms: pos.perms.clone(),
            },
        );

        pos_cache_by_role_id.insert(
            pos.role_id.clone(),
            CachedPosition {
                id: pos.id,
                name: pos.name.clone(),
                role_id: pos.role_id.clone(),
                index: pos.index,
                perms: pos.perms.clone(),
            },
        );

        pos_cache_by_name.insert(
            pos.name.clone(),
            CachedPosition {
                id: pos.id,
                name: pos.name.clone(),
                role_id: pos.role_id.clone(),
                index: pos.index,
                perms: pos.perms.clone(),
            },
        );
    }

    // Also, get the current list of staff members from the db
    let staff = sqlx::query!(
        "SELECT user_id, positions, perm_overrides, no_autosync, unaccounted FROM staff_members FOR UPDATE"
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Error while getting staff members: {:?}", e))?;

    let mut staff_override_perms = HashMap::new();
    let mut staff_noautosync = HashSet::new();
    let mut staff_unaccounted = HashSet::new();

    for member in staff.iter() {
        staff_override_perms.insert(member.user_id.clone(), member.perm_overrides.clone());
    }

    // This keeps track of any user_ids not accounted for
    let mut unaccounted_user_ids = {
        let mut unaccounted_user_ids = HashSet::new();

        for user in staff.iter() {
            if user.no_autosync {
                staff_noautosync.insert(user.user_id.clone());
                continue;
            }

            // Known unaccounted (but may have been reaccepted)
            if user.unaccounted {
                staff_unaccounted.insert(user.user_id.clone());                
            }

            unaccounted_user_ids.insert(user.user_id.clone());
        }

        unaccounted_user_ids
    };

    // To speed up operations, make a map of id to the positions beforehand itself
    let member_pos_cache = {
        let mut member_pos_cache = HashMap::new();

        for member in staff {
            if member.no_autosync {
                continue;
            }

            let mut positions = Vec::new();

            for pos in member.positions {
                positions.push(pos);
            }

            member_pos_cache.insert(member.user_id.clone(), positions);
        }

        member_pos_cache
    };

    for user in staff_resync {
        // Skip if the user is in the noautosync list
        if staff_noautosync.contains(&user.user_id.to_string()) {
            continue;
        }

        let mut is_on_db: bool = true;
        let user_positions_db = match member_pos_cache.get(&user.user_id.to_string()) {
            Some(p) => {
                // Create a hashset of the positions
                let mut positions = HashSet::new();

                for pos in p {
                    // Garbage Collection Step: Remove if not in the cache
                    if !pos_cache_by_id.contains_key(pos) {
                        sqlx::query!(
                            "UPDATE staff_members SET positions = array_remove(positions, $1) WHERE user_id = $2",
                            pos,
                            user.user_id.to_string()
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| format!("Error while removing staff member position: {:?}", e))?;
                    } else {
                        positions.insert(*pos);
                    }
                }

                positions
            }
            None => {
                is_on_db = false;
                HashSet::new()
            } // Empty/no perms
        };

        let mut user_positions = HashSet::new();

        // Special case: owner
        if crate::config::CONFIG.owners.contains(&user.user_id) {
            let o_pos = pos_cache_by_name.get("owner");

            if let Some(o_pos) = o_pos {
                user_positions.insert(o_pos.id);
            }
        }

        for role in user.roles {
            if let Some(pos) = pos_cache_by_role_id.get(&role) {
                if pos.name == *"owner" {
                    // Skip owner, its a special case
                    continue;
                }
                if !user_positions.contains(&pos.id) {
                    user_positions.insert(pos.id);
                }
            }
        }

        // Compare user_positions_db and user_positions
        if user_positions
            .symmetric_difference(&user_positions_db)
            .count()
            > 0
        {
            // Concatenate the positions
            let mut user_positions_vec = Vec::new();
            for pos in user_positions.iter() {
                user_positions_vec.push(*pos);
            }

            if is_on_db {
                sqlx::query!(
                    "UPDATE staff_members SET positions = $1, unaccounted = false WHERE user_id = $2",
                    &user_positions_vec,
                    user.user_id.to_string()
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Error while updating staff member positions: {:?}", e))?;
            } else {
                sqlx::query!(
                    "INSERT INTO staff_members (user_id, positions) VALUES ($1, $2)",
                    user.user_id.to_string(),
                    &user_positions_vec,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e: sqlx::Error| format!("Error while inserting staff member positions: {:?}", e))?;
            }

            // Get the position with the highest index
            let mut lowest_index = i32::MAX;

            for pos in user_positions.iter() {
                if let Some(pos) = pos_cache_by_id.get(pos) {
                    if pos.index < lowest_index {
                        lowest_index = pos.index;
                    }
                }
            }

            // Positions are different, update the db and set new perms replacing any overrides
            let mut old_sp = perms::StaffPermissions {
                user_positions: vec![],
                perm_overrides: vec![],
            };

            for pos in user_positions_db.iter() {
                if let Some(pos) = pos_cache_by_id.get(pos) {
                    old_sp.user_positions.push(perms::PartialStaffPosition {
                        id: pos.id.hyphenated().to_string(),
                        index: pos.index,
                        perms: pos.perms.clone(),
                    });
                }
            }

            let mut new_sp = perms::StaffPermissions {
                user_positions: vec![],
                perm_overrides: vec![],
            };

            for pos in user_positions.iter() {
                if let Some(pos) = pos_cache_by_id.get(pos) {
                    new_sp.user_positions.push(perms::PartialStaffPosition {
                        id: pos.id.hyphenated().to_string(),
                        index: pos.index,
                        perms: pos.perms.clone(),
                    });
                }
            }

            // Add in the override_perms
            if let Some(perms) = staff_override_perms.get(&user.user_id.to_string()) {
                old_sp.perm_overrides = perms.clone();
                new_sp.perm_overrides = perms.clone();
            }            

            // Concatenate the positions
            let mut user_positions_vec = Vec::new();
            for pos in user_positions.iter() {
                user_positions_vec.push(*pos);
            }

            crate::config::CONFIG
                .channels
                .staff_logs
                .send_message(
                    &cache_http.http,
                    CreateMessage::new().embeds(vec![CreateEmbed::new()
                        .title("Staff Permissions Resync")
                        .description(format!("Updated staff permissions for <@{}>", user.user_id))
                        .field(
                            "Old Positions",
                            {
                                let mut positions = Vec::new();
                                for pos in user_positions_db.iter() {
                                    if let Some(pos) = pos_cache_by_id.get(pos) {
                                        positions.push(format!("- ``{}``", pos));
                                    } else {
                                        positions.push(format!("- Unknown Position: {}", pos));
                                    }
                                }

                                if positions.is_empty() {
                                    positions.push("None".to_string());
                                }

                                positions.join("\n")
                            },
                            false,
                        )
                        .field(
                            "New Positions",
                            {
                                let mut positions = Vec::new();
                                for pos in user_positions.iter() {
                                    if let Some(pos) = pos_cache_by_id.get(pos) {
                                        positions.push(format!("- ``{}``", pos));
                                    } else {
                                        positions.push(format!("- Unknown Position: {}", pos));
                                    }
                                }

                                if positions.is_empty() {
                                    positions.push("None".to_string());
                                }

                                positions.join("\n")
                            },
                            false,
                        )
                        .field(
                            "Old Permissions",
                            {
                                let operms = old_sp.resolve();
                                let mut perms = Vec::new();
                                for perm in operms.iter() {
                                    perms.push(format!("- ``{}``", perm));
                                }

                                if perms.is_empty() {
                                    perms.push("None".to_string());
                                }

                                perms.join("\n")
                            },
                            false,
                        )
                        .field(
                            "New Permissions",
                            {
                                let nperms = new_sp.resolve();
                                let mut perms = Vec::new();
                                for perm in nperms.iter() {
                                    perms.push(format!("- ``{}``", perm));
                                }

                                if perms.is_empty() {
                                    perms.push("None".to_string());
                                }

                                perms.join("\n")
                            },
                            false,
                        )]),
                )
                .await
                .map_err(|e| format!("Error while sending staff logs message: {:?}", e))?;
        }

        unaccounted_user_ids.remove(&user.user_id.to_string());
    }

    // Now, remove any unaccounted users
    for user_id in unaccounted_user_ids {
        // Skip if the user is in the noautosync list *OR* if they are known unaccounted
        if staff_noautosync.contains(&user_id) || staff_unaccounted.contains(&user_id) {
            continue;
        }

        let delete = if let Some(p) = staff_override_perms.get(&user_id) {
            p.is_empty()
        } else {
            true
        };

        if delete {
            sqlx::query!("DELETE FROM staff_members WHERE user_id = $1", user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while removing unaccounted staff member: {:?}", e))?;
        } else {
            sqlx::query!("UPDATE staff_members SET positions = '{}', unaccounted = true WHERE user_id = $1", user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while updating unaccounted staff member: {:?}", e))?;
        }

        let mut old_sp = perms::StaffPermissions {
            user_positions: vec![],
            perm_overrides: vec![],
        };

        for pos in member_pos_cache.get(&user_id).unwrap() {
            if let Some(pos) = pos_cache_by_id.get(pos) {
                old_sp.user_positions.push(perms::PartialStaffPosition {
                    id: pos.id.hyphenated().to_string(),
                    index: pos.index,
                    perms: pos.perms.clone(),
                });
            }
        }

        if let Some(perms) = staff_override_perms.get(&user_id) {
            old_sp.perm_overrides = perms.clone();
        }            

        if delete {
            crate::config::CONFIG.channels.staff_logs.send_message(
                &cache_http.http,
                    CreateMessage::new().embeds(vec![
                        CreateEmbed::new()
                        .title("Staff Permissions Resync")
                        .description(format!(
                            "Removed unaccounted staff member <@{}> as they are no longer in the staff server.",
                            user_id
                        ))
                        .field(
                            "Old Positions", 
                            {
                                let mut positions = Vec::new();
                                for pos in member_pos_cache.get(&user_id).unwrap() {
                                    if let Some(pos) = pos_cache_by_id.get(pos) {
                                        positions.push(format!("- ``{}``", pos));
                                    } else {
                                        positions.push(format!("- Unknown Position: {}", pos));
                                    }
                                }
    
                                if positions.is_empty() {
                                    positions.push("None".to_string());
                                }
                                
                                positions.join("\n")
                            },
                            false
                        )
                        .field(
                            "Old Permissions", 
                            {
                                let operms = old_sp.resolve();
                                let mut perms = Vec::new();
                                for perm in operms.iter() {
                                    perms.push(format!("- ``{}``", perm));
                                }
    
                                if perms.is_empty() {
                                    perms.push("None".to_string());
                                }
                                
                               perms.join("\n")
                            },
                            false
                        )
                    ]),
            )
            .await
            .map_err(|e| format!("Error while sending staff logs message: {:?}", e))?;    
        } else {
            crate::config::CONFIG.channels.staff_logs.send_message(
                &cache_http.http,
                    CreateMessage::new().embeds(vec![
                        CreateEmbed::new()
                        .title("Staff Permissions Resync")
                        .description(format!(
                            "Updated unaccounted staff member <@{}> as they are no longer in the staff server but have permission overrides.",
                            user_id
                        ))
                        .field(
                            "Old Positions", 
                            {
                                let mut positions = Vec::new();
                                for pos in member_pos_cache.get(&user_id).unwrap() {
                                    if let Some(pos) = pos_cache_by_id.get(pos) {
                                        positions.push(format!("- ``{}``", pos));
                                    } else {
                                        positions.push(format!("- Unknown Position: {}", pos));
                                    }
                                }
    
                                if positions.is_empty() {
                                    positions.push("None".to_string());
                                }
                                
                                positions.join("\n")
                            },
                            false
                        )
                        .field(
                            "Old Permissions", 
                            {
                                let operms = old_sp.resolve();
                                let mut perms = Vec::new();
                                for perm in operms.iter() {
                                    perms.push(format!("- ``{}``", perm));
                                }
    
                                if perms.is_empty() {
                                    perms.push("None".to_string());
                                }
                                
                               perms.join("\n")
                            },
                            false
                        )
                    ]),
            )
            .await
            .map_err(|e| format!("Error while sending staff logs message: {:?}", e))?;    
        }
    }

    // Commit the transaction
    tx.commit()
        .await
        .map_err(|e| format!("Error while committing transaction: {:?}", e))?;

    Ok(())
}

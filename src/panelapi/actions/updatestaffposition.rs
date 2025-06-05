use std::str::FromStr;

use crate::panelapi::auth::{check_auth, get_staff_member};
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::staff_positions::{
    CorrespondingServer, StaffPosition, StaffPositionAction,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use kittycat::perms::{self, Permission};
use serenity::all::RoleId;
use strum::VariantNames;

pub async fn update_staff_position(
    state: &AppState,
    login_token: String,
    action: StaffPositionAction,
) -> Result<Response, Error> {
    let auth_data = check_auth(&state.pool, &login_token)
        .await
        .map_err(Error::new)?;

    match action {
        StaffPositionAction::ListPositions => {
            let pos = sqlx::query!("SELECT id, name, role_id, perms, corresponding_roles, icon, index, created_at FROM staff_positions ORDER BY index ASC")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| format!("Error while getting staff positions {}", e))
        .map_err(Error::new)?;

            let mut positions = Vec::new();

            for position_data in pos {
                positions.push(StaffPosition {
                    id: position_data.id.hyphenated().to_string(),
                    name: position_data.name,
                    role_id: position_data.role_id,
                    perms: position_data.perms,
                    corresponding_roles: serde_json::from_value(position_data.corresponding_roles)
                        .map_err(Error::new)?,
                    icon: position_data.icon,
                    index: position_data.index,
                    created_at: position_data.created_at,
                });
            }

            Ok((StatusCode::OK, Json(positions)).into_response())
        }
        StaffPositionAction::SwapIndex { a, b } => {
            // Get permissions
            let sm = get_staff_member(&state.pool, &state.cache_http, &auth_data.user_id)
                .await
                .map_err(Error::new)?;

            if !perms::has_perm(&sm.resolved_perms, &"staff_positions.swap_index".into()) {
                return Ok((
                StatusCode::FORBIDDEN,
                "You do not have permission to swap indexes of staff positions [staff_positions.swap_index]".to_string(),
            )
                .into_response());
            }

            // Get the lowest index permission of the member
            let mut sm_lowest_index = i32::MAX;

            for perm in &sm.positions {
                if perm.index < sm_lowest_index {
                    sm_lowest_index = perm.index;
                }
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            let index_a = sqlx::query!("SELECT index FROM staff_positions WHERE id::text = $1", a)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| format!("Error while getting lower position {}", e))
                .map_err(Error::new)?
                .index;

            // Get the higher staff positions index
            let index_b = sqlx::query!("SELECT index FROM staff_positions WHERE id::text = $1", b)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| format!("Error while getting higher position {}", e))
                .map_err(Error::new)?
                .index;

            if index_a == index_b {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Positions have the same index".to_string(),
                )
                    .into_response());
            }

            // If either a or b is lower than the lowest index of the member, then error
            if index_a <= sm_lowest_index || index_b <= sm_lowest_index {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "Either 'a' or 'b' is lower than the lowest index of the member".to_string(),
                )
                    .into_response());
            }

            // Swap the indexes
            sqlx::query!(
                "UPDATE staff_positions SET index = $1 WHERE id::text = $2",
                index_b,
                a
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while updating lower position {}", e))
            .map_err(Error::new)?;

            sqlx::query!(
                "UPDATE staff_positions SET index = $1 WHERE id::text = $2",
                index_a,
                b
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while updating higher position {}", e))
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        StaffPositionAction::SetIndex { id, index } => {
            let uuid = sqlx::types::uuid::Uuid::parse_str(&id).map_err(Error::new)?;

            // Get permissions
            let sm = get_staff_member(&state.pool, &state.cache_http, &auth_data.user_id)
                .await
                .map_err(Error::new)?;

            if !perms::has_perm(&sm.resolved_perms, &"staff_positions.set_index".into()) {
                return Ok((
                StatusCode::FORBIDDEN,
                "You do not have permission to set the indexes of staff positions [staff_positions.set_index]".to_string(),
            )
                .into_response());
            }

            if index < 0 {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Index cannot be lower than 0".to_string(),
                )
                    .into_response());
            }

            // Get the lowest index permission of the member
            let mut sm_lowest_index = i32::MAX;

            for perm in &sm.positions {
                if perm.index < sm_lowest_index {
                    sm_lowest_index = perm.index;
                }
            }

            if index <= sm_lowest_index {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "Index to set is lower than or equal to the lowest index of the staff member"
                        .to_string(),
                )
                    .into_response());
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            let curr_index = sqlx::query!("SELECT index FROM staff_positions WHERE id = $1", uuid)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| format!("Error while getting position {}", e))
                .map_err(Error::new)?
                .index;

            // If the current index is lower than the lowest index of the member, then error
            if curr_index <= sm_lowest_index {
                return Ok((
                StatusCode::FORBIDDEN,
                "Current index of position is lower than or equal to the lowest index of the staff member".to_string(),
            )
                .into_response());
            }

            // Shift indexes one lower
            sqlx::query!(
                "UPDATE staff_positions SET index = index + 1 WHERE index >= $1",
                index
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while shifting indexes {}", e))
            .map_err(Error::new)?;

            // Set the index
            sqlx::query!(
                "UPDATE staff_positions SET index = $1 WHERE id = $2",
                index,
                uuid
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while updating position {}", e))
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        StaffPositionAction::CreatePosition {
            name,
            role_id,
            perms,
            index,
            corresponding_roles,
            icon,
        } => {
            // Get permissions
            let sm = get_staff_member(&state.pool, &state.cache_http, &auth_data.user_id)
                .await
                .map_err(Error::new)?;

            if !perms::has_perm(&sm.resolved_perms, &"staff_positions.create".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to create staff positions [staff_positions.create]"
                        .to_string(),
                )
                    .into_response());
            }

            if index < 0 {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Index cannot be lower than 0".to_string(),
                )
                    .into_response());
            }

            // Get the lowest index permission of the member
            let mut sm_lowest_index = i32::MAX;

            for perm in &sm.positions {
                if perm.index < sm_lowest_index {
                    sm_lowest_index = perm.index;
                }
            }

            if index <= sm_lowest_index {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "Index is lower than or equal to the lowest index of the staff member"
                        .to_string(),
                )
                    .into_response());
            }

            // Shift indexes one lower
            let mut tx = state.pool.begin().await.map_err(Error::new)?;
            sqlx::query!(
                "UPDATE staff_positions SET index = index + 1 WHERE index >= $1",
                index
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while shifting indexes {}", e))
            .map_err(Error::new)?;

            // Ensure role id exists on the staff server
            let role_id_snow = role_id.parse::<RoleId>().map_err(Error::new)?;
            let role_exists = {
                let guild = state
                    .cache_http
                    .cache
                    .guild(crate::config::CONFIG.servers.staff);

                if let Some(guild) = guild {
                    guild.roles.get(&role_id_snow).is_some()
                } else {
                    false
                }
            };

            if !role_exists {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Role does not exist on the staff server".to_string(),
                )
                    .into_response());
            }

            // Ensure all corresponding_roles exist on the named server if
            for role in corresponding_roles.iter() {
                let Ok(corr_server) = CorrespondingServer::from_str(&role.name) else {
                    return Ok((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "Server {} is not a supported corresponding role. Supported: {:#?}",
                            role.name,
                            CorrespondingServer::VARIANTS
                        ),
                    )
                        .into_response());
                };
                let role_id_snow = role.value.parse::<RoleId>().map_err(Error::new)?;

                let role_exists = {
                    let guild = state.cache_http.cache.guild(corr_server.get_id());

                    if let Some(guild) = guild {
                        guild.roles.get(&role_id_snow).is_some()
                    } else {
                        false
                    }
                };

                if !role_exists {
                    return Ok((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "Role {} does not exist on the server {}",
                            role_id_snow,
                            corr_server.get_id()
                        ),
                    )
                        .into_response());
                }
            }

            // Create the position
            sqlx::query!(
            "INSERT INTO staff_positions (name, perms, corresponding_roles, icon, role_id, index) VALUES ($1, $2, $3, $4, $5, $6)",
            name,
            &perms,
            serde_json::to_value(corresponding_roles).map_err(Error::new)?,
            icon,
            role_id,
            index,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error while updating position {}", e))
        .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        StaffPositionAction::EditPosition {
            id,
            name,
            role_id,
            perms,
            corresponding_roles,
            icon,
        } => {
            let uuid = sqlx::types::uuid::Uuid::parse_str(&id).map_err(Error::new)?;

            // Get permissions
            let sm = get_staff_member(&state.pool, &state.cache_http, &auth_data.user_id)
                .await
                .map_err(Error::new)?;

            if !perms::has_perm(&sm.resolved_perms, &"staff_positions.edit".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to edit staff positions [staff_positions.edit]"
                        .to_string(),
                )
                    .into_response());
            }

            // Get the lowest index permission of the member
            let mut sm_lowest_index = i32::MAX;

            for perm in &sm.positions {
                if perm.index < sm_lowest_index {
                    sm_lowest_index = perm.index;
                }
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            // Get the index and current permissions of the position
            let index = sqlx::query!(
                "SELECT perms, index, role_id FROM staff_positions WHERE id = $1 FOR UPDATE",
                uuid
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Error while getting position {}", e))
            .map_err(Error::new)?;

            // If the index is lower than the lowest index of the member, then error
            if index.index <= sm_lowest_index {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "Index is lower than the lowest index of the member".to_string(),
                )
                    .into_response());
            }

            // Check perms
            if let Err(e) = perms::check_patch_changes(
                &sm.resolved_perms,
                &index
                    .perms
                    .iter()
                    .map(|x| Permission::from_string(x))
                    .collect::<Vec<Permission>>(),
                &perms
                    .iter()
                    .map(|x| Permission::from_string(x))
                    .collect::<Vec<Permission>>(),
            ) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    format!(
                        "You do not have permission to edit the following perms: {}",
                        e
                    ),
                )
                    .into_response());
            }

            // Ensure role id exists on the staff server
            let role_id_snow = role_id.parse::<RoleId>().map_err(Error::new)?;
            let role_exists = {
                let guild = state
                    .cache_http
                    .cache
                    .guild(crate::config::CONFIG.servers.staff);

                if let Some(guild) = guild {
                    guild.roles.get(&role_id_snow).is_some()
                } else {
                    false
                }
            };

            if !role_exists {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Role does not exist on the staff server".to_string(),
                )
                    .into_response());
            }

            // Ensure all corresponding_roles exist on the named server if
            for role in corresponding_roles.iter() {
                let Ok(corr_server) = CorrespondingServer::from_str(&role.name) else {
                    return Ok((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "Server {} is not a supported corresponding role. Supported: {:#?}",
                            role.name,
                            CorrespondingServer::VARIANTS
                        ),
                    )
                        .into_response());
                };
                let role_id_snow = role.value.parse::<RoleId>().map_err(Error::new)?;

                let role_exists = {
                    let guild = state.cache_http.cache.guild(corr_server.get_id());

                    if let Some(guild) = guild {
                        guild.roles.get(&role_id_snow).is_some()
                    } else {
                        false
                    }
                };

                if !role_exists {
                    return Ok((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "Role {} does not exist on the server {}",
                            role_id_snow,
                            corr_server.get_id()
                        ),
                    )
                        .into_response());
                }
            }

            // Update the position
            sqlx::query!(
            "UPDATE staff_positions SET name = $1, perms = $2, corresponding_roles = $3, role_id = $4, icon = $5 WHERE id = $6",
            name,
            &perms,
            serde_json::to_value(corresponding_roles).map_err(Error::new)?,
            role_id,
            icon,
            uuid
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error while updating position {}", e))
        .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        StaffPositionAction::DeletePosition { id } => {
            let uuid = sqlx::types::uuid::Uuid::parse_str(&id).map_err(Error::new)?;

            // Get permissions
            let sm = get_staff_member(&state.pool, &state.cache_http, &auth_data.user_id)
                .await
                .map_err(Error::new)?;

            if !perms::has_perm(&sm.resolved_perms, &"staff_positions.delete".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to delete staff positions [staff_positions.delete]"
                        .to_string(),
                )
                    .into_response());
            }

            // Get the lowest index permission of the member
            let mut sm_lowest_index = i32::MAX;

            for perm in &sm.positions {
                if perm.index < sm_lowest_index {
                    sm_lowest_index = perm.index;
                }
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            // Get the index and current permissions of the position
            let index = sqlx::query!(
                "SELECT perms, index, role_id FROM staff_positions WHERE id = $1 FOR UPDATE",
                uuid
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Error while getting position {}", e))
            .map_err(Error::new)?;

            // If the index is lower than the lowest index of the member, then error
            if index.index <= sm_lowest_index {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "Index is lower than the lowest index of the member".to_string(),
                )
                    .into_response());
            }

            // Check perms
            if let Err(e) = perms::check_patch_changes(
                &sm.resolved_perms,
                &index
                    .perms
                    .iter()
                    .map(|x| Permission::from_string(x))
                    .collect::<Vec<Permission>>(),
                &Vec::new(),
            ) {
                return Ok((
                StatusCode::FORBIDDEN,
                format!("You do not have permission to edit the following perms [neeed to delete position]: {}", e),
            )
                .into_response());
            }

            // Delete the position
            sqlx::query!("DELETE FROM staff_positions WHERE id = $1", uuid)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Error while deleting position {}", e))
                .map_err(Error::new)?;

            // Shift back indexes one lower
            sqlx::query!(
                "UPDATE staff_positions SET index = index - 1 WHERE index > $1",
                index.index
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while shifting indexes {}", e))
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
    }
}

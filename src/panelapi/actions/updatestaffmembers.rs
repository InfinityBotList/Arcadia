use crate::panelapi::auth::{check_auth, get_staff_member};
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::staff_members::StaffMemberAction;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use kittycat::perms::{self, Permission};

pub async fn update_staff_members(
    state: &AppState,
    login_token: String,
    action: StaffMemberAction,
) -> Result<Response, Error> {
    let auth_data = check_auth(&state.pool, &login_token)
        .await
        .map_err(Error::new)?;

    match action {
        StaffMemberAction::ListMembers => {
            let ids = sqlx::query!("SELECT user_id FROM staff_members")
                .fetch_all(&state.pool)
                .await
                .map_err(|e| format!("Error while getting staff members {}", e))
                .map_err(Error::new)?;

            let mut members = Vec::new();

            for id in ids {
                let member = get_staff_member(&state.pool, &state.cache_http, &id.user_id)
                    .await
                    .map_err(Error::new)?;

                members.push(member);
            }

            Ok((StatusCode::OK, Json(members)).into_response())
        }
        StaffMemberAction::EditMember {
            user_id,
            perm_overrides,
            no_autosync,
            unaccounted,
        } => {
            // Get permissions
            let sm = get_staff_member(&state.pool, &state.cache_http, &auth_data.user_id)
                .await
                .map_err(Error::new)?;

            // Get permissions of target
            let sm_target = get_staff_member(&state.pool, &state.cache_http, &user_id)
                .await
                .map_err(Error::new)?;

            if !perms::has_perm(&sm.resolved_perms, &"staff_members.edit".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to edit staff members [staff_members.edit]"
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

            // Get the lowest index permission of the target
            let mut sm_target_lowest_index = i32::MAX;

            for perm in &sm_target.positions {
                if perm.index < sm_target_lowest_index {
                    sm_target_lowest_index = perm.index;
                }
            }

            // If the target has a lower index than the member, then error
            if sm_target_lowest_index < sm_lowest_index {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "Target has a lower index than the member".to_string(),
                )
                    .into_response());
            }

            let perm_overrides = perm_overrides
                .iter()
                .map(|x| Permission::from_string(x))
                .collect::<Vec<Permission>>();

            // Check perms with resolved perms following addition of overrides
            let new_resolved_perms = perms::StaffPermissions {
                perm_overrides: perm_overrides.clone(),
                ..sm_target.staff_permission
            }
            .resolve();

            if let Err(e) = perms::check_patch_changes(
                &sm.resolved_perms,
                &sm_target.resolved_perms,
                &new_resolved_perms,
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

            // Then update
            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            // Lock the member for update
            sqlx::query!("SELECT perm_overrides, no_autosync, unaccounted FROM staff_members WHERE user_id = $1 FOR UPDATE", user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Error while getting member {}", e))
        .map_err(Error::new)?;

            // Update the member
            sqlx::query!("UPDATE staff_members SET perm_overrides = $1, no_autosync = $2, unaccounted = $3 WHERE user_id = $4",
            &perm_overrides.iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            no_autosync,
            unaccounted,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error while updating member {}", e))
        .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
    }
}

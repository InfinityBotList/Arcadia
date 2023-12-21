use crate::Error;
use kittycat::perms::{StaffPermissions, PartialStaffPosition};
use sqlx::PgPool;

use super::types::{auth::AuthData, webcore::{StaffPosition, StaffMember}};

/// Checks auth, but does not ensure active sessions
pub async fn check_auth_insecure(pool: &PgPool, token: &str) -> Result<AuthData, Error> {
    // Delete expired auths
    sqlx::query!("DELETE FROM staffpanel__authchain WHERE created_at < NOW() - INTERVAL '1 hour'")
        .execute(pool)
        .await?;

    // Delete expired auths that are inactive
    sqlx::query!(
        "DELETE FROM staffpanel__authchain WHERE state = 'pending' AND created_at < NOW() - INTERVAL '5 minutes'"
    )
    .execute(pool)
    .await?;

    let count = sqlx::query!(
        "SELECT COUNT(*) FROM staffpanel__authchain WHERE token = $1",
        token
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap_or(0);

    if count == 0 {
        return Err("identityExpired".into());
    }

    let rec = sqlx::query!(
        "SELECT user_id, created_at, state FROM staffpanel__authchain WHERE token = $1",
        token
    )
    .fetch_one(pool)
    .await?;

    Ok(AuthData {
        user_id: rec.user_id,
        created_at: rec.created_at.timestamp(),
        state: rec.state,
    })
}

/// Checks auth, and ensures active sessions
///
/// Equivalent to `check_auth_insecure`, and rec.status != "active"
pub async fn check_auth(pool: &PgPool, token: &str) -> Result<AuthData, Error> {
    let rec = check_auth_insecure(pool, token).await?;

    if rec.state != "active" {
        return Err("sessionNotActive".into());
    }

    Ok(rec)
}

/// Returns the data of a staff member
pub async fn get_staff_member(pool: &PgPool, user_id: &str) -> Result<StaffMember, Error> {
    let data = sqlx::query!(
        "SELECT positions, perm_overrides, no_autosync, created_at FROM staff_members WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e: sqlx::Error| format!("Error while getting staff perms of user {}: {}", user_id, e))?;

    let pos = sqlx::query!("SELECT id, name, role_id, perms, index, created_at FROM staff_positions WHERE id = ANY($1)", &data.positions)
    .fetch_all(pool)
    .await
    .map_err(|e: sqlx::Error| format!("Error while getting positions of user {}: {}", user_id, e))?;

    let mut positions = Vec::new();
    let sp = StaffPermissions {
        user_positions: pos.iter().map(|p| PartialStaffPosition {
            id: p.id.hyphenated().to_string(),
            index: p.index,
            perms: p.perms.clone(),
        }).collect(),
        perm_overrides: data.perm_overrides.clone(),
    };

    for position_data in pos {
        positions.push(StaffPosition {
            id: position_data.id.hyphenated().to_string(),
            name: position_data.name,
            role_id: position_data.role_id,
            perms: position_data.perms,
            index: position_data.index,
            created_at: position_data.created_at,
        });
    }

    Ok(
        StaffMember {
            user_id: user_id.to_string().clone(),
            positions,
            perm_overrides: data.perm_overrides,
            resolved_perms: sp.resolve(),
            no_autosync: data.no_autosync,
            created_at: data.created_at,
        }
    )    
}
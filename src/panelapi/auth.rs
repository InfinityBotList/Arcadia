use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use crate::Error;
use ts_rs::TS;

use super::types::{Capability, PanelPerms};

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = ".generated/AuthData.ts")]
pub struct AuthData {
    pub user_id: String,
    pub created_at: i64,
    pub state: String,
}

/// Checks auth, but does not ensure active sessions
pub async fn check_auth_insecure(pool: &PgPool, token: &str) -> Result<AuthData, Error> {
    // Delete expired auths
    sqlx::query!(
        "DELETE FROM staffpanel__authchain WHERE created_at < NOW() - INTERVAL '30 minutes'"
    )
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

pub async fn get_user_perms(pool: &PgPool, user_id: &str) -> Result<PanelPerms, Error> {
    let perms = sqlx::query!(
        "SELECT staff, admin, hadmin, ibldev, iblhdev, owner FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(
        PanelPerms {
            staff: perms.staff,
            admin: perms.admin,
            hadmin: perms.hadmin,
            ibldev: perms.ibldev,
            iblhdev: perms.iblhdev,
            owner: perms.owner,
        }
    )
}

/// Returns the capabilities of a user
/// 
/// NOTE 1: Server list and bot management capability not enabled right now
/// 
/// NOTE 2: in the future, capabilities can be limited based on user info/perms as well
pub async fn get_capabilities(pool: &PgPool, token: &str) -> Result<Vec<Capability>, Error> {
    let auth_data = check_auth(pool, token).await?;

    let perms = get_user_perms(pool, &auth_data.user_id).await?;

    let mut capabilities = Vec::new();

    if perms.staff {
        capabilities.push(Capability::ViewBotQueue);
        capabilities.push(Capability::Rpc);
    }

    Ok(capabilities)
}

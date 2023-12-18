use crate::Error;
use sqlx::PgPool;

use super::types::auth::AuthData;

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

pub async fn get_user_perms(pool: &PgPool, login_token: &str) -> Result<Vec<String>, Error> {
    let rec = check_auth(pool, login_token).await?;

    let perms = sqlx::query!(
        "SELECT perms FROM staff_members WHERE user_id = $1",
        rec.user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(perms.perms)
}

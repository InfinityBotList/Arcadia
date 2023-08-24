use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use crate::Error;
use ts_rs::TS;

use super::types::Capability;

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = ".generated/AuthData.ts")]
pub struct AuthData {
    pub user_id: String,
    pub created_at: i64,
}

pub async fn check_auth(pool: &PgPool, token: &str) -> Result<AuthData, Error> {
    // Delete expired auths
    sqlx::query!(
        "DELETE FROM rpc__panelauthchain WHERE created_at < NOW() - INTERVAL '1 hour'"
    )
    .execute(pool)
    .await?;

    let count = sqlx::query!(
        "SELECT COUNT(*) FROM rpc__panelauthchain WHERE token = $1",
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
        "SELECT user_id, created_at FROM rpc__panelauthchain WHERE token = $1",
        token
    )
    .fetch_one(pool)
    .await?;

    Ok(AuthData {
        user_id: rec.user_id,
        created_at: rec.created_at.timestamp(),
    })
}

/// Returns the capabilities of a user
/// 
/// NOTE 1: Server list and bot management capability not enabled right now
/// 
/// NOTE 2: in the future, capabilities can be limited based on user info/perms as well
pub async fn get_capabilities(pool: &PgPool, token: &str) -> Result<Vec<Capability>, Error> {
    check_auth(pool, token).await?;

    Ok(vec![
        Capability::Rpc,
    ])
}

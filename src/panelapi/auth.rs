use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use crate::Error;
use ts_rs::TS;

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

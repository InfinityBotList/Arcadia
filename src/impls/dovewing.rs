use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = ".generated/PartialUser.ts")]
pub struct PartialUser {
    pub username: String,
    pub display_name: String,
    pub bot: bool,
    pub avatar: String,
}

pub async fn get_partial_user(pool: &PgPool, user_id: &str) -> Result<PartialUser, crate::Error> {
    let rec = sqlx::query!(
        "SELECT username, display_name, avatar, bot FROM internal_user_cache__discord WHERE id = $1",
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(PartialUser {
        username: rec.username,
        display_name: rec.display_name,
        bot: rec.bot,
        avatar: rec.avatar,
    })
}

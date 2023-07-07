use serde::{Serialize, Deserialize};
use sqlx::PgPool;

#[derive(Clone, Serialize, Deserialize)]
pub struct PartialUser {
    pub username: String,
    pub display_name: String,
    pub bot: bool
}

pub async fn get_partial_user(
    pool: &PgPool,
    user_id: &str,
) -> Result<PartialUser, crate::Error> {
    let rec = sqlx::query!(
        "SELECT username, display_name, bot FROM internal_user_cache__discord WHERE id = $1",
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(PartialUser {
        username: rec.username,
        display_name: rec.display_name,
        bot: rec.bot
    })
}
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Data, Error};

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = ".generated/AuthPayload.ts")]
pub struct AuthPayload {
    pub user_id: String,
    pub token: String,
}

impl AuthPayload {
    pub async fn authorize(&self, data: &Data) -> Result<(), Error> {
        // Ensure they are staff
        let res = sqlx::query!(
            "SELECT staff FROM users WHERE user_id = $1 AND api_token = $2",
            self.user_id,
            self.token
        )
        .fetch_one(&data.pool)
        .await
        .map_err(|_| "User not found")?;

        if res.staff {
            Ok(())
        } else {
            Err("User is not staff".into())
        }
    }
}

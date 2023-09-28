use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(
    Serialize, Deserialize, PartialEq, EnumString, ToSchema, TS, EnumVariantNames, Clone, Default,
)]
#[ts(export, export_to = ".generated/TargetType.ts")]
pub enum TargetType {
    #[default]
    Bot,
    Server,
    Team,
    Pack,
}

impl Display for TargetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Bot => write!(f, "bot"),
            TargetType::Server => write!(f, "server"),
            TargetType::Team => write!(f, "team"),
            TargetType::Pack => write!(f, "pack"),
        }
    }
}

/* TODO/TOUSE
impl TargetType {
    pub async fn get_vanity(pool: &PgPool, vanity_ref: sqlx::types::Uuid) -> Result<String, crate::Error> {
        let rec = sqlx::query!(
            "SELECT code::text FROM vanity WHERE itag = $1",
            vanity_ref
        )
        .fetch_one(pool)
        .await?;

        let Some(code) = rec.code else {
            return Err("No code found for vanity".into());
        };

        Ok(code)
    }
}*/
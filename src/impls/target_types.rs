use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
    User,
}

impl Display for TargetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Bot => write!(f, "bot"),
            TargetType::Server => write!(f, "server"),
            TargetType::Team => write!(f, "team"),
            TargetType::Pack => write!(f, "pack"),
            TargetType::User => write!(f, "user"),
        }
    }
}

impl TargetType {
    #[allow(dead_code)] // TODO: Use this basic support
    pub fn supports_votes(&self) -> bool {
        match self {
            TargetType::Bot => true,
            TargetType::Server => true,
            TargetType::Team => true,
            TargetType::Pack => true,
            TargetType::User => false,
        }
    }

    #[allow(dead_code)] // TODO: Use this basic support
    pub fn id(&self) -> String {
        match self {
            TargetType::Bot => "bot_id".to_string(),
            TargetType::Server => "server_id".to_string(),
            TargetType::Team => "id".to_string(),
            TargetType::Pack => "url".to_string(),
            TargetType::User => "user_id".to_string(),
        }
    }
}

impl TargetType {
    #[allow(dead_code)] // TODO: Implement this/use this basic support
    pub async fn get_vanity(
        pool: &PgPool,
        vanity_ref: sqlx::types::Uuid,
    ) -> Result<String, crate::Error> {
        let rec = sqlx::query!("SELECT code::text FROM vanity WHERE itag = $1", vanity_ref)
            .fetch_one(pool)
            .await?;

        let Some(code) = rec.code else {
            return Err("No code found for vanity".into());
        };

        Ok(code)
    }
}

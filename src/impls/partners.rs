use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ts_rs::TS;
use utoipa::ToSchema;
use serde::{Serialize, Deserialize};
use crate::Error;

use super::link::Link;

#[derive(Serialize, Deserialize, PartialEq, TS, Clone, Default, ToSchema)]
#[ts(export, export_to = ".generated/Partner.ts")]
pub struct Partner {
    pub id: String,
    pub name: String,
    pub image: String,
    pub short: String,
    pub links: Vec<Link>,
    pub r#type: String,
    pub created_at: DateTime<Utc>,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, PartialEq, TS, Clone, Default, ToSchema)]
#[ts(export, export_to = ".generated/Partners.ts")]
pub struct Partners {
    pub partners: Vec<Partner>
}

impl Partners {
    pub async fn fetch(pool: &PgPool) -> Result<Self, Error> {
        let prec = sqlx::query!("SELECT * FROM partners")
            .fetch_all(pool)
            .await?;

        let mut partners = Vec::new();

        for partner in prec {
            partners.push(
                Partner {
                    id: partner.id,
                    name: partner.name,
                    image: partner.image,
                    short: partner.short,
                    links: serde_json::from_value(partner.links)?,
                    r#type: partner.r#type,
                    created_at: partner.created_at,
                    user_id: partner.user_id,
                }
            )
        }

        Ok(Self {
            partners
        })
    }
}
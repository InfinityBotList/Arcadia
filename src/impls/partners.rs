use crate::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;
use utoipa::ToSchema;

use super::link::Link;

#[derive(Serialize, Deserialize, PartialEq, TS, Clone, Default, ToSchema)]
#[ts(export, export_to = ".generated/Partner.ts")]
pub struct Partner {
    pub id: String,
    pub name: String,
    pub image_type: String,
    pub short: String,
    pub links: Vec<Link>,
    pub r#type: String,
    pub created_at: DateTime<Utc>,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, PartialEq, TS, Clone, Default, ToSchema)]
#[ts(export, export_to = ".generated/PartnerType.ts")]
pub struct PartnerType {
    pub id: String,
    pub name: String,
    pub short: String,
    pub icon: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, PartialEq, TS, Clone, Default, ToSchema)]
#[ts(export, export_to = ".generated/Partners.ts")]
pub struct Partners {
    pub partners: Vec<Partner>,
    pub partner_types: Vec<PartnerType>,
}

impl Partners {
    pub async fn fetch(pool: &PgPool) -> Result<Self, Error> {
        let prec = sqlx::query!(
            "SELECT id, name, image_type, short, links, type, created_at, user_id FROM partners"
        )
        .fetch_all(pool)
        .await?;

        let mut partners = Vec::new();

        for partner in prec {
            partners.push(Partner {
                id: partner.id,
                name: partner.name,
                image_type: partner.image_type,
                short: partner.short,
                links: serde_json::from_value(partner.links)?,
                r#type: partner.r#type,
                created_at: partner.created_at,
                user_id: partner.user_id,
            })
        }

        let ptrec = sqlx::query!("SELECT id, name, short, icon, created_at FROM partner_types")
            .fetch_all(pool)
            .await?;

        let mut partner_types = Vec::new();

        for partner_type in ptrec {
            partner_types.push(PartnerType {
                id: partner_type.id,
                name: partner_type.name,
                short: partner_type.short,
                icon: partner_type.icon,
                created_at: partner_type.created_at,
            })
        }

        Ok(Self {
            partners,
            partner_types,
        })
    }
}

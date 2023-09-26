use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use crate::impls::link::Link;

#[derive(Serialize, Deserialize, PartialEq, TS, Clone, Default, ToSchema)]
#[ts(export, export_to = ".generated/CreatePartner.ts")]
pub struct CreatePartner {
    pub id: String,
    pub name: String,
    pub image_type: String,
    pub short: String,
    pub links: Vec<Link>,
    pub r#type: String,
    pub user_id: String,
}

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
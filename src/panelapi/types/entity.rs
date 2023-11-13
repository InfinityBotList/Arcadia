use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use strum_macros::{Display, EnumVariantNames};
use crate::impls::dovewing::PartialUser;

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/PartialBot.ts")]
pub struct PartialBot {
    pub bot_id: String,
    pub user: PartialUser,
    pub short: String,
    pub r#type: String,
    pub votes: i32,
    pub shards: i32,
    pub library: String,
    pub invite_clicks: i32,
    pub clicks: i32,
    pub servers: i32,
    pub claimed_by: Option<String>,
    pub last_claimed: Option<chrono::DateTime<chrono::Utc>>,
    pub approval_note: String,
    pub mentionable: Vec<String>,
    pub invite: String,
    pub client_id: String,
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/PartialServer.ts")]
pub struct PartialServer {
    pub server_id: String,
    pub name: String,
    pub avatar: String,
    pub total_members: i32,
    pub online_members: i32,
    pub short: String,
    pub r#type: String,
    pub votes: i32,
    pub invite_clicks: i32,
    pub clicks: i32,
    pub nsfw: bool,
    pub tags: Vec<String>,
    pub premium: bool,
    pub claimed_by: Option<String>,
    pub last_claimed: Option<chrono::DateTime<chrono::Utc>>,
    pub mentionable: Vec<String>,
}

#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    TS,
    EnumVariantNames,
    Display,
    Clone,
)]
#[ts(export, export_to = ".generated/PartialEntity.ts")]
pub enum PartialEntity {
    Bot(PartialBot),
    Server(PartialServer),
}

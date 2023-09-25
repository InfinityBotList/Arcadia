use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::impls::dovewing::PartialUser;

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/QueueBot.ts")]
pub struct QueueBot {
    pub bot_id: String,
    pub client_id: String,
    pub user: PartialUser,
    pub claimed_by: Option<String>,
    pub approval_note: String,
    pub short: String,
    pub mentionable: Vec<String>,
    pub invite: String,
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/SearchBot.ts")]
pub struct SearchBot {
    pub bot_id: String,
    pub client_id: String,
    pub user: PartialUser,
    pub claimed_by: Option<String>,
    pub r#type: String,
    pub approval_note: String,
    pub short: String,
    pub mentionable: Vec<String>,
    pub invite: String,
}

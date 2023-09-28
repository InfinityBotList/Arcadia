use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use strum_macros::{Display, EnumVariantNames};
use crate::impls::dovewing::PartialUser;

#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    TS,
    EnumVariantNames,
    Display,
    Clone,
    PartialEq,
)]
#[ts(export, export_to = ".generated/PartialEntity.ts")]
pub enum PartialEntity {
    Bot {
        bot_id: String,
        user: PartialUser,
        short: String,
        r#type: String,
        votes: i32,
        shards: i32,
        library: String,
        invite_clicks: i32,
        clicks: i32,
        servers: i32,
        claimed_by: Option<String>,
        approval_note: String,
        mentionable: Vec<String>,
        invite: String,
        client_id: String,
    },
    Server {
        server_id: String,
        name: String,
        avatar: String,
        total_members: i32,
        online_members: i32,
        short: String,
        r#type: String,
        votes: i32,
        invite_clicks: i32,
        clicks: i32,
        nsfw: bool,
        tags: Vec<String>,
        premium: bool,
        banner: Option<String>,
    },
}

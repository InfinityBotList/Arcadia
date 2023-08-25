use strum_macros::{EnumString, Display, EnumVariantNames};
use ts_rs::TS;
use serde::{Serialize, Deserialize};
use utoipa::ToSchema;

use crate::{impls::dovewing::PartialUser, config::Servers};

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/PanelPerms.ts")]
pub struct PanelPerms {
    pub staff: bool,
    pub admin: bool,
    pub hadmin: bool,
    pub ibldev: bool,
    pub iblhdev: bool,
    pub owner: bool,
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/InstanceConfig.ts")]
/// Represents a user
pub struct InstanceConfig {
    /// Description of the instance
    pub description: String,
    /// Instance URL
    pub instance_url: String,
    /// Path at which all queries can be made
    pub query: String,
}

#[derive(Serialize, Deserialize, ToSchema, TS, EnumString, EnumVariantNames, Display, Clone)]
#[ts(export, export_to = ".generated/Capability.ts")]
pub enum Capability {
    /// RPC capability
    Rpc,
    /// View bot queue capability
    ViewBotQueue,
    /// Server listing capability
    ServerList,
    /// Bot management capability
    BotManagement,
}

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
#[ts(export, export_to = ".generated/CoreConstants.ts")]
pub struct CoreConstants {
    /// URL to the main site (reed is used here currently)
    pub frontend_url: String,
    /// Infernoplex URL
    pub infernoplex_url: String,
    /// Servers
    pub servers: Servers
}
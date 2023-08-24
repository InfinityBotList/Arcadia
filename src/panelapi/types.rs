use strum_macros::{EnumString, Display, EnumVariantNames};
use ts_rs::TS;
use serde::{Serialize, Deserialize};
use utoipa::ToSchema;

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
    /// Server listing capability
    ServerList,
    /// Bot management capability
    BotManagement,
}
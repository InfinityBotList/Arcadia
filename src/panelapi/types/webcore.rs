use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;
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
    /// Any warnings for the instance
    pub warnings: Vec<String>,
}

#[derive(
    Serialize, Deserialize, ToSchema, TS, EnumString, EnumVariantNames, Display, Clone, PartialEq,
)]
#[ts(export, export_to = ".generated/Capability.ts")]
pub enum Capability {
    /// RPC capability
    Rpc,
    /// View bot queue capability
    ViewBotQueue,
    /// Server management capability
    ServerManagement,
    /// Bot management capability
    BotManagement,
    /// Ability to manage partners
    PartnerManagement,
    /// Ability to add assets to the CDN
    CdnManagement,
    /// Ability to manage changelogs
    ChangelogManagement,
    /// Ability to view applications [not yet implemented]
    ViewApps,
    /// Ability to manage applications [not yet implemented]
    ManageApps,
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/CoreConstants.ts")]
pub struct CoreConstants {
    /// URL to the main site (reed is used here currently)
    pub frontend_url: String,
    /// Infernoplex URL
    pub infernoplex_url: String,
    /// CDN URL
    pub cdn_url: String,
    /// Popplio URL
    pub popplio_url: String,
    /// Servers
    pub servers: PanelServers,
}

/// Same as CONFIG.servers but using strings instead of NonZeroU64s
#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/PanelServers.ts")]
pub struct PanelServers {
    pub main: String,
    pub staff: String,
    pub testing: String,
}

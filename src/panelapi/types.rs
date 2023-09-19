use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::{
    impls::{dovewing::PartialUser, target_types::TargetType},
    rpc::core::{RPCField, RPCPerms},
};

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
}

#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    TS,
    EnumString,
    EnumVariantNames,
    Display,
    Clone,
    PartialEq,
    Default,
)]
#[ts(export, export_to = ".generated/CdnAssetAction.ts")]
pub enum CdnAssetAction {
    /// List entries in path
    ///
    /// Using this ignores the `name` field
    #[default]
    ListPath,
    /// Read an asset
    ReadFile,
    /// Creates a new folder
    CreateFolder,
    /// Creates an asset
    ///
    /// The file itself must not already exist
    AddFile {
        /// Allow overwrite of existing file
        overwrite: bool,
        /// Base 64 encoded file contents
        contents: String,
    },
    /// Copies an asset already on the server to a new location
    CopyFile {
        /// Allow overwrite of existing file
        overwrite: bool,
        /// Delete the original file
        delete_original: bool,
        /// Path to copy to
        copy_to: String,
    },
    /// Delete asset or folder
    Delete,
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/CdnAssetItem.ts")]
pub struct CdnAssetItem {
    /// Name of the asset
    pub name: String,
    /// Path of the asset
    pub path: String,
    /// Size of the asset
    pub size: u64,
    /// Last modified time of the asset as unix epoch
    pub last_modified: u64,
    /// Whether the asset is a directory
    pub is_dir: bool,
    /// Permissions of the asset
    pub permissions: u32,
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

/// MFA Login Secret Data
#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/MfaLoginSecret.ts")]
pub struct MfaLoginSecret {
    pub secret: String,
    pub otp_url: String,
    pub qr_code: String,
}

/// MFA Login Data
#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/MfaLogin.ts")]
pub struct MfaLogin {
    pub info: Option<MfaLoginSecret>,
}

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = ".generated/RPCWebAction.ts")]
pub struct RPCWebAction {
    /// ID of the RPC action
    pub id: String,
    /// Label of the RPC action
    pub label: String,
    /// Description of the RPC action
    pub description: String,
    /// Fields of the RPC action
    pub fields: Vec<RPCField>,
    /// Target types supported by the RPC action
    pub supported_target_types: Vec<TargetType>,
    /// Permissions required to use the RPC action
    pub needs_perms: RPCPerms,
}

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/StaffPosition.ts")]
pub struct StaffPosition {
    /// The ID of the position
    pub id: String,
    /// The name of the position
    pub name: String,
    /// The role id associated with this position on Discord
    pub role_id: String,
    /// The preset permissions of this position
    pub perms: Vec<String>,
    /// The index of the position, higher means further up
    pub index: i32,
    /// When the staff position was created/added
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/StaffMember.ts")]
pub struct StaffMember {
    /// The id of the user
    pub user_id: String,
    /// The positions of the staff member
    pub positions: Vec<StaffPosition>,
    /// The permissions available to the member
    pub perms: Vec<String>,
    /// Whether or not the member is 'frozen' and cannot be updated in resyncs
    pub no_autosync: bool,
    /// When the staff member was created/added
    pub created_at: chrono::DateTime<chrono::Utc>,
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
    /// HTMLSanitize URL
    pub htmlsanitize_url: String,
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

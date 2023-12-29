use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::impls::target_types::TargetType;

use super::{staff_members::StaffMember, auth::AuthData};

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

/// StartAuth contains the needed data to begin a login
#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/StartAuth.ts")]
pub struct StartAuth {
    /// The URL to redirect to
    pub login_url: String,
    /// The request scope
    pub scope: String,
    /// Response Scope is just a key to allow for the frontend to verify the backend as compatible
    pub response_scope: String,
}

/// Hello contains the configuration event needed for the panel to work
#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/Hello.ts")]
pub struct Hello {
    pub instance_config: InstanceConfig,
    pub auth_data: AuthData,
    pub staff_member: StaffMember,
    pub core_constants: CoreConstants,
    pub target_types: Vec<TargetType>,
}
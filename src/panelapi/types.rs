use crate::impls::dovewing::PartialUser;
use ts_rs::TS;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/PanelUserDetails.ts")]
/// Represents a user
pub struct PanelUserDetails {
    pub user: PartialUser,
    pub staff: bool,
    pub admin: bool,
    pub hadmin: bool,
    pub ibldev: bool,
    pub iblhdev: bool,
    pub owner: bool,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/PanelUserDetails.ts")]
/// Represents a user
pub struct InstanceConfig {
    /// Description of the instance
    pub description: String,
    /// Instance URL
    pub instance_url: String,
    /// Path at which all queries can be made
    pub query: String,
}

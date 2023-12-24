use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

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
#[ts(export, export_to = ".generated/StaffPositionAction.ts")]
pub enum StaffPositionAction {
    /// List all current positions
    #[default]
    ListPositions,
    /// Swap the index of two staff positions (A and B) such that the indexes change from (Ia, Ib) -> (Ib, Ia)
    SwapIndex {
        /// Staff Position A
        a: String,
        /// Staff Position B
        b: String,
    },
    /// Create a new staff position
    CreatePosition {
        /// The name of the position
        name: String,
        /// The role id associated with this position on Discord
        role_id: String,
        /// The preset permissions of this position
        perms: Vec<String>,
        /// The index of the position, higher means further down on hierarchy
        index: i32,
    },
    /// Edit staff position
    /// 
    /// To edit index, use the `SwapIndex` action
    EditPosition {
        /// The ID of the position
        id: String,
        /// The name of the position
        name: String,
        /// The role id associated with this position on Discord
        role_id: String,
        /// The preset permissions of this position
        perms: Vec<String>,
    },
}

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
    /// The index of the position, higher means further down on hierarchy
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
    /// The permission overrides of the staff member
    pub perm_overrides: Vec<String>,
    /// The resolved permissions available to the member
    pub resolved_perms: Vec<String>,
    /// Whether or not the member is 'frozen' and cannot be updated in resyncs
    pub no_autosync: bool,
    /// When the staff member was created/added
    pub created_at: chrono::DateTime<chrono::Utc>,
}
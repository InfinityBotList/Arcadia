use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;
use super::staff_positions::StaffPosition;

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
#[ts(export, export_to = ".generated/StaffMemberAction.ts")]
pub enum StaffMemberAction {
    /// List all current members
    #[default]
    ListMembers,
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
    /// Whether or not the member is 'known' to be 'unaccounted' for
    pub unaccounted: bool,
    /// When the staff member was created/added
    pub created_at: chrono::DateTime<chrono::Utc>,
}

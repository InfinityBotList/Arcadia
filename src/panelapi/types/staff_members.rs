use super::staff_disciplinary::StaffDisciplinary;
use crate::impls::dovewing::PlatformUser;
use kittycat::perms::Permission;
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

    /// Edit a staff member
    EditMember {
        /// The user id of the member
        user_id: String,

        /// The permission overrides of the staff member
        perm_overrides: Vec<String>,

        /// Whether or not to autosync the member
        no_autosync: bool,

        /// Whether or not the member is 'known' to be 'unaccounted' for
        unaccounted: bool,
    },
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/StaffMember.ts")]
pub struct StaffMember {
    /// The id of the user
    pub user_id: String,
    /// The user object of the staff member
    pub user: PlatformUser,
    /// The positions of the staff member
    pub positions: Vec<StaffPosition>,
    /// The disciplinary actions recieved by the member
    pub disciplinaries: Vec<StaffDisciplinary>,
    /// The permission overrides of the staff member
    pub perm_overrides: Vec<String>,
    #[serde(skip)]
    #[ts(skip)]
    pub resolved_perms: Vec<Permission>,
    #[serde(rename = "resolved_perms")]
    #[ts(rename = "resolved_perms")]
    /// The resolved permissions available to the member
    pub resolved_perms_kc: Vec<String>,
    /// Whether or not the member is 'frozen' and cannot be updated in resyncs
    pub no_autosync: bool,
    /// Whether or not the member is 'known' to be 'unaccounted' for
    pub unaccounted: bool,
    /// Whether or not the members MFA is verified or not
    pub mfa_verified: bool,
    /// When the staff member was created/added
    pub created_at: chrono::DateTime<chrono::Utc>,
}

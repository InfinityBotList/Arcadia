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
#[ts(export, export_to = ".generated/StaffDisciplinaryTypeAction.ts")]
pub enum StaffDisciplinaryTypeAction {
    /// List all current staff disciplinary types
    #[default]
    ListDisciplinaryTypes,

    /// Create a staff disciplinary types
    CreateDisciplinaryType {
        /// The id of the action
        id: String,

        /// Name of the action
        name: String,

        /// Description of the action
        description: String,

        /// Whether or not the action is self-assignable
        self_assignable: bool,

        /// The permission limits of the action
        perm_limits: Vec<String>,

        /// Whether the perm_limits of the disciplinary are 'additory'
        ///
        /// E.g. do the perms get combined with the users perms, or do they replace them
        additory: bool,

        /// Whether or not this type needs approval
        needs_approval: bool,

        /// Maximum expiry in seconds of the action/type
        max_expiry: Option<f64>,
    },

    /// Edit a staff disciplinary types
    EditDisciplinaryType {
        /// The id of the action
        id: String,

        /// Name of the action
        name: String,

        /// Description of the action
        description: String,

        /// Whether or not the action is self-assignable
        self_assignable: bool,

        /// The permission limits of the action
        perm_limits: Vec<String>,

        /// Whether the perm_limits of the disciplinary are 'additory'
        ///
        /// E.g. do the perms get combined with the users perms, or do they replace them
        additory: bool,

        /// Whether or not this type needs approval
        needs_approval: bool,

        /// Maximum expiry in seconds of the action/type
        max_expiry: Option<f64>,
    },

    /// Delete a staff disciplinary type
    DeleteDisciplinaryType {
        /// The id of the action
        id: String,
    },
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/StaffDisciplinaryType.ts")]
pub struct StaffDisciplinaryType {
    /// The id of the type
    pub id: String,

    /// Name of the action
    pub name: String,

    /// Description of the type
    pub description: String,

    /// Whether or not the type is self-assignable
    pub self_assignable: bool,

    /// The permission limits of the type
    pub perm_limits: Vec<String>,

    /// Whether the perm_limits of the disciplinary are 'additory'
    ///
    /// E.g. do the perms get combined with the users perms, or do they replace them
    pub additory: bool,

    /// Whether or not this type needs approval
    pub needs_approval: bool,

    /// Maximum expiry in seconds of the action/type
    pub max_expiry: Option<f64>,

    /// When the staff disciplinary type was created/added
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/StaffDisciplinary.ts")]
pub struct StaffDisciplinary {
    /// The ID of the position
    pub id: String,

    /// The user ID who recieved of the disciplinary action
    pub user_id: String,

    /// When the staff disciplinary action was created/added
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When the disciplinary action expires, in seconds
    pub expires_at: Option<i64>,

    /// The title of the disciplinary action report
    pub title: String,

    /// The description of the disciplinary action report
    pub description: String,

    /// The type of the disciplinary
    pub r#type: StaffDisciplinaryType,
}

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
#[ts(export, export_to = ".generated/ChangelogAction.ts")]
pub enum ChangelogAction {
    /// List changelog entries
    ///
    /// Note that all staff members can list the changelog
    #[default]
    ListEntries,

    /// Create a new changelog entry
    CreateEntry {
        /// Version for the changelog entry to add
        version: String,
        /// Extra description for the version, if applicable
        extra_description: String,
        /// Whether or not this is a prerelease
        prerelease: bool,
        /// Added features for the version
        added: Vec<String>,
        /// Updated features for the version
        updated: Vec<String>,
        /// Removed features for the version
        removed: Vec<String>,
    },

    /// Update a changelog entry
    UpdateEntry {
        /// Version for the changelog entry to update
        version: String,
        /// Extra description for the version, if applicable
        extra_description: String,
        /// Github HTML for the version, if applicable
        github_html: Option<String>,
        /// Whether or not this is a prerelease
        prerelease: bool,
        /// Added features for the version
        added: Vec<String>,
        /// Updated features for the version
        updated: Vec<String>,
        /// Removed features for the version
        removed: Vec<String>,
        /// Whether or not to publish the version
        published: bool,
    },

    /// Delete a changelog entry
    DeleteEntry {
        /// Version for the changelog entry to delete
        version: String,
    },
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/ChangelogEntry.ts")]
pub struct ChangelogEntry {
    pub version: String,
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    pub github_html: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub extra_description: String,
    pub prerelease: bool,
    pub published: bool,
}

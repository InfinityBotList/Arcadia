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
    #[default]
    ListEntries,
}

/* 
	Version          string      `db:"version" json:"version" validate:"required" description:"The version for the changelog entry. (4.3.0 etc.)"`
	ExtraDescription string      `db:"extra_description" json:"extra_description" description:"The extra description for the version, if applicable"`
	GithubHTML       pgtype.Text `db:"github_html" json:"github_html" description:"The Github-backed HTML for the changelog entry."`
	Prerelease       bool        `db:"prerelease" json:"prerelease" description:"Whether or not this is a prerelease."`
	Added            []string    `db:"added" json:"added" validate:"required" description:"The added features for the version."`
	Updated          []string    `db:"updated" json:"updated" validate:"required" description:"The changed features for the version."`
	Removed          []string    `db:"removed" json:"removed" validate:"required" description:"The removed features for the version."`
*/

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
}
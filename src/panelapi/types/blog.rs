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
#[ts(export, export_to = ".generated/BlogAction.ts")]
pub enum BlogAction {
    /// List blog entries
    ///
    /// Note that all staff members can list all blog entries
    #[default]
    ListEntries,

    /// Create a new blog entry
    CreateEntry {
        /// Slug/vanity code for the blog entry to add
        slug: String,
        /// Title of the blog entry to add
        title: String,
        /// Description of the blog entry
        description: String,
        /// The content of the blog entry
        content: String,
        /// The tags for the blog entry
        tags: Vec<String>,
    },

    /// Updates a blog entry
    UpdateEntry {
        /// The internal id of the blog post
        itag: String,
        /// Slug/vanity code for the blog entry to edit
        slug: String,
        /// Title of the blog entry to edit
        title: String,
        /// Description of the blog entry
        description: String,
        /// The content of the blog entry
        content: String,
        /// The tags for the blog entry
        tags: Vec<String>,
        /// Whether or not the blog entry is a draft or not
        draft: bool,
    },

    /// Delete a blog entry
    DeleteEntry {
        /// ID of the entry to delete
        itag: String,
    },
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/BlogPost.ts")]
pub struct BlogPost {
    /// The internal id of the blog post
    pub itag: String,
    /// Slug/vanity code for the blog entry to add
    pub slug: String,
    /// Title of the blog entry to add
    pub title: String,
    /// Description of the blog entry
    pub description: String,
    /// User ID who made the blog entry
    pub user_id: String,
    /// When the blog post was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// The content of the blog entry
    pub content: String,
    /// Whether or not the blog entry is a draft or not
    pub draft: bool,
    /// The tags for the blog entry
    pub tags: Vec<String>,
}

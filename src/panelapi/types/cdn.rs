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
#[ts(export, export_to = ".generated/CdnAssetAction.ts")]
pub enum CdnAssetAction {
    /// List entries in path
    ///
    /// Using this ignores the `name` field
    #[default]
    ListPath,
    /// Read an asset
    ReadFile,
    /// Creates a new folder
    CreateFolder,
    /// Creates an asset
    ///
    /// The file itself must not already exist
    AddFile {
        /// Allow overwrite of existing file
        overwrite: bool,
        /// Base 64 encoded file contents uploaded as multiple chunks with an ID associated with each chunk
        chunks: Vec<String>,
        /// SHA512 hash of the file
        sha512: String,
    },
    /// Copies an asset already on the server to a new location
    CopyFile {
        /// Allow overwrite of existing file
        overwrite: bool,
        /// Delete the original file
        delete_original: bool,
        /// Path to copy to
        copy_to: String,
    },
    /// Delete asset or folder
    Delete,
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/CdnAssetItem.ts")]
pub struct CdnAssetItem {
    /// Name of the asset
    pub name: String,
    /// Path of the asset
    pub path: String,
    /// Size of the asset
    pub size: u64,
    /// Last modified time of the asset as unix epoch
    pub last_modified: u64,
    /// Whether the asset is a directory
    pub is_dir: bool,
    /// Permissions of the asset
    pub permissions: u32,
}

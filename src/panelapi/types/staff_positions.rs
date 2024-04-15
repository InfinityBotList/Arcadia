use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::impls::link::Link;

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
    /// Sets the new index of a staff position
    SetIndex {
        /// The ID of the position
        id: String,
        /// The new index of the position
        index: i32,
    },
    /// Create a new staff position
    CreatePosition {
        /// The name of the position
        name: String,
        /// The role id associated with this position on Discord [staff server]
        role_id: String,
        /// The corresponding role on discord
        corresponding_roles: Vec<Link>,
        /// The preset permissions of this position
        perms: Vec<String>,
        /// The icon of the position
        icon: String,
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
        /// The corresponding role on discord
        corresponding_roles: Vec<Link>,
        /// The preset permissions of this position
        perms: Vec<String>,
        /// The icon of the position
        icon: String,
    },
    /// Delete a staff position
    DeletePosition {
        /// The ID of the position
        id: String,
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
    /// Corresponding roles of the position
    pub corresponding_roles: Vec<Link>,
    /// The icon of the position
    pub icon: String,
    /// The index of the position, higher means further down on hierarchy
    pub index: i32,
    /// When the staff position was created/added
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, TS, EnumVariantNames, Display, Clone, PartialEq)]
#[ts(export, export_to = ".generated/CorrespondingServer.ts")]
pub enum CorrespondingServer {
    Main,
    Testing,
    Staff,
}

impl FromStr for CorrespondingServer {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "main" => Ok(CorrespondingServer::Main),
            "testing" => Ok(CorrespondingServer::Testing),
            "staff" => Ok(CorrespondingServer::Staff),
            _ => Err(format!("Invalid corresponding server: {}", s).into()),
        }
    }
}

impl CorrespondingServer {
    pub fn get_id(&self) -> serenity::all::GuildId {
        match self {
            CorrespondingServer::Main => crate::config::CONFIG.servers.main,
            CorrespondingServer::Testing => crate::config::CONFIG.servers.testing,
            CorrespondingServer::Staff => crate::config::CONFIG.servers.staff,
        }
    }
}

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = ".generated/RPCLogEntry.ts")]
pub struct RPCLogEntry {
    /// ID of the RPC log entry
    pub id: String,
    /// User ID of the entry
    pub user_id: String,
    /// The method used
    pub method: String,
    /// The state/status of the rpc action
    pub state: String,
    /// The data provided
    pub data: serde_json::Value,
    /// When the entry was created at
    pub created_at: chrono::DateTime<chrono::Utc>,
}

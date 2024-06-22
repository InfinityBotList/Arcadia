use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, TS, Clone, PartialEq)]
#[ts(export, export_to = ".generated/BaseAnalytics.ts")]
pub struct BaseAnalytics {
    pub bot_counts: std::collections::HashMap<String, i64>,
    pub server_counts: std::collections::HashMap<String, i64>,
    pub ticket_counts: std::collections::HashMap<String, i64>,
    pub total_users: i64,
    pub changelogs_count: i64,
}

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, PartialEq, TS, Clone, Default, ToSchema)]
#[ts(export, export_to = ".generated/Link.ts")]
pub struct Link {
    pub name: String,
    pub value: String,
}

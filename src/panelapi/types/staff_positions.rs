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
    }
}
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

/// Vote credits are tier based through slabs
/// (e.g.)For the following tiers 
/// 
/// - Tier 1: 100 votes at 0.10 cents 
/// - Tier 2: 200 votes at 0.05 cents 
/// - Tier 3: 50 votes at 0.025 cents
/// 
/// Would mean 625 votes would be split as the following:
/// 
/// 100 votes: 0.10 cents [Tier 1]
/// Next 200 votes: 0.05 cents [Tier 2]
/// Next 50 votes: 0.025 cents [Tier 3]
/// Last 275 votes: 0.025 cents [last tier used at end of tiering]
#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/VoteCreditTier.ts")]
pub struct VoteCreditTier {
    /// The ID of the tier
    pub id: String,
    /// The position of the tier
    pub position: i32,
    /// The cents per vote
    pub cents: f64,
    /// The number of votes in this tier
    pub votes: i32,
    /// The time the tier was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

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
#[ts(export, export_to = ".generated/VoteCreditTierAction.ts")]
pub enum VoteCreditTierAction {
    /// List all current vote credit tiers
    #[default]
    ListTiers,
    /// Create a new vote credit tier
    CreateTier {
        /// The ID of the tier
        id: String,
        /// The position of the tier
        position: i32,
        /// The cents per vote
        cents: f64,
        /// The number of votes in this tier
        votes: i32,
    },
    /// Edit vote credit tier
    ///
    /// To edit index, use the `SwapIndex` action
    EditTier {
        /// The ID of the tier
        id: String,
        /// The position of the tier
        position: i32,
        /// The cents per vote
        cents: f64,
        /// The number of votes in this tier
        votes: i32,
    },
    /// Delete a vote credit tier
    DeleteTier {
        /// The ID of the tier
        id: String,
    },

    /*
        /// Swap the index of two vote credit tiers (A and B) such that the indexes change from (Ia, Ib) -> (Ib, Ia)
    SwapIndex {
        /// Vote Credit Tier A
        a: String,
        /// Vote Credit Tier B
        b: String,
    },
    /// Sets the new index of a vote credit tier
    SetIndex {
        /// The ID of the tier
        id: String,
        /// The new index of the tier
        index: i32,
    },
     */
}
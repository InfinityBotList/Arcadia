use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

/// Shop items are items that can be purchased by users on the shop
#[derive(Serialize, Deserialize, TS, Clone, ToSchema)]
#[ts(export, export_to = ".generated/ShopItemBenefit.ts")]
pub struct ShopItemBenefit {
    /// The ID of the shop item benefit
    pub id: String,
    /// The friendly name of the shop item benefit
    pub name: String,
    /// The description of the shop item benefit
    pub description: String,
    /// The time the shop item benefit was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// The time the shop item benefit was last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Who created the shop item benefit
    pub created_by: String,
    /// Who last updated the shop item benefit
    pub updated_by: String,
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
#[ts(export, export_to = ".generated/ShopItemBenefitAction.ts")]
pub enum ShopItemBenefitAction {
    /// List all current shop item benefits
    #[default]
    List,
    /// Create a new shop item benefit
    Create {
        /// The ID of the shop item benefit
        id: String,
        /// The friendly name of the shop item benefit
        name: String,
        /// The description of the shop item benefit
        description: String,
    },
    /// Edit a shop item benefit
    Edit {
        /// The ID of the shop item benefit
        id: String,
        /// The friendly name of the shop item benefit
        name: String,
        /// The description of the shop item benefit
        description: String,
    },
    /// Deletes a shop item benefit
    Delete {
        /// The ID of the shop item benefit
        id: String,
    },
}

/// Shop items are items that can be purchased by users on the shop
#[derive(Serialize, Deserialize, TS, Clone, ToSchema)]
#[ts(export, export_to = ".generated/ShopItem.ts")]
pub struct ShopItem {
    /// The ID of the shop item
    pub id: String,
    /// The friendly name of the shop item
    pub name: String,
    /// The description of the shop item
    pub description: String,
    /// The cents the shop item costs
    pub cents: f64,
    /// The target type
    pub target_types: Vec<String>,
    /// The benefits of the shop item
    pub benefits: Vec<String>,
    /// The number of hours the shop item lasts for
    pub duration: i32,
    /// The time the shop item was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// The time the shop item was last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Who created the shop item
    pub created_by: String,
    /// Who last updated the shop item
    pub updated_by: String,
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
#[ts(export, export_to = ".generated/ShopItemAction.ts")]
pub enum ShopItemAction {
    /// List all current shop items
    #[default]
    List,
    /// Create a new shop item
    Create {
        /// The ID of the shop item
        id: String,
        /// The friendly name of the shop item
        name: String,
        /// The description of the shop item
        description: String,
        /// The cents the shop item costs
        cents: f64,
        /// The target type
        target_types: Vec<String>,
        /// The benefits of the shop item
        benefits: Vec<String>,
        /// The number of hours the shop item lasts for
        duration: i32,
    },
    /// Edit a shop item
    Edit {
        /// The ID of the shop item
        id: String,
        /// The friendly name of the shop item
        name: String,
        /// The description of the shop item
        description: String,
        /// The cents the shop item costs
        cents: f64,
        /// The target type
        target_types: Vec<String>,
        /// The benefits of the shop item
        benefits: Vec<String>,
        /// The number of hours the shop item lasts for
        duration: i32,
    },
    /// Deletes a shop item
    Delete {
        /// The ID of the shop item
        id: String,
    },
}

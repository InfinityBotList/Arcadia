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
    /// The target types the benefit can be applied to
    pub target_types: Vec<String>,
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
        /// The target types the benefit can be applied to
        target_types: Vec<String>,
    },
    /// Edit a shop item benefit
    Edit {
        /// The ID of the shop item benefit
        id: String,
        /// The friendly name of the shop item benefit
        name: String,
        /// The description of the shop item benefit
        description: String,
        /// The target types the benefit can be applied to
        target_types: Vec<String>,
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

#[derive(Serialize, Deserialize, TS, Clone, ToSchema)]
#[ts(export, export_to = ".generated/ShopCoupon.ts")]
pub struct ShopCoupon {
    /// The ID of the shop coupon
    pub id: String,
    /// The code of the shop coupon
    pub code: String,
    /// Whether the shop coupon is publicly viewable in the API or not
    pub public: bool,
    /// The maximum number of times the shop coupon can be used, if None, the shop coupon can be used an unlimited number of times
    pub max_uses: Option<i32>,
    /// The time the shop coupon was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// The user who created the shop coupon
    pub created_by: String,
    /// The time the shop coupon was last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// The user who last updated the shop coupon
    pub updated_by: String,
    /// The number of hours that must be waited to reuse the shop coupon
    ///
    /// If None, the shop coupon can be reused immediately without wait
    pub reuse_wait_duration: Option<i32>,
    /// The number of hours the shop coupon expires in
    ///
    /// If None, the shop coupon never expires
    pub expiry: Option<i32>,
    /// The items the shop coupon is applicable to
    ///
    /// If empty, the shop coupon is applicable to all items
    pub applicable_items: Vec<String>,
    /// The cents the shop coupon is worth
    ///
    /// If none, the shop coupon is worth the total cost of the items it is being used on
    pub cents: Option<f64>,
    /// The requirements to use the shop coupon
    pub requirements: Vec<String>,
    /// The users the coupon is applicable for
    ///
    /// If empty, the coupon is applicable to all users
    pub allowed_users: Vec<String>,
    /// Whether or not the coupon is usable or not
    pub usable: bool,
    /// The target types the coupon can be applied to
    ///
    /// If empty, the coupon is applicable to all target types
    pub target_types: Vec<String>,
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
#[ts(export, export_to = ".generated/ShopCouponAction.ts")]
pub enum ShopCouponAction {
    /// List all current shop coupons
    #[default]
    List,

    /// Create a new shop coupon
    Create {
        /// The ID of the shop coupon
        id: String,
        /// The code of the shop coupon
        code: String,
        /// Whether the shop coupon is publicly viewable in the API or not
        public: bool,
        /// The maximum number of times the shop coupon can be used, if None, the shop coupon can be used an unlimited number of times
        max_uses: Option<i32>,
        /// The number of hours that must be waited to reuse the shop coupon
        ///
        /// If None, the shop coupon can be reused immediately without wait
        reuse_wait_duration: Option<i32>,
        /// The number of hours the shop coupon expires in
        ///
        /// If None, the shop coupon never expires
        expiry: Option<i32>,
        /// The items the shop coupon is applicable to
        ///
        /// If empty, the shop coupon is applicable to all items
        applicable_items: Vec<String>,
        /// The cents the shop coupon is worth
        ///
        /// If none, the shop coupon is worth the total cost of the items it is being used on
        cents: Option<f64>,
        /// The requirements to use the shop coupon
        requirements: Vec<String>,
        /// The users the coupon is applicable for
        ///
        /// If empty, the coupon is applicable to all users
        allowed_users: Vec<String>,
        /// Whether or not the coupon is usable or not
        usable: bool,
        /// The target types the coupon can be applied to
        ///
        /// If empty, the coupon is applicable to all target types
        target_types: Vec<String>,
    },
    /// Edit a shop coupon
    Edit {
        /// The ID of the shop coupon
        id: String,
        /// The code of the shop coupon
        code: String,
        /// Whether the shop coupon is publicly viewable in the API or not
        public: bool,
        /// The maximum number of times the shop coupon can be used, if None, the shop coupon can be used an unlimited number of times
        max_uses: Option<i32>,
        /// The number of hours that must be waited to reuse the shop coupon
        ///
        /// If None, the shop coupon can be reused immediately without wait
        reuse_wait_duration: Option<i32>,
        /// The number of hours the shop coupon expires in
        ///
        /// If None, the shop coupon never expires
        expiry: Option<i32>,
        /// The items the shop coupon is applicable to
        ///
        /// If empty, the shop coupon is applicable to all items
        applicable_items: Vec<String>,
        /// The cents the shop coupon is worth
        ///
        /// If none, the shop coupon is worth the total cost of the items it is being used on
        cents: Option<f64>,
        /// The requirements to use the shop coupon
        requirements: Vec<String>,
        /// The users the coupon is applicable for
        ///
        /// If empty, the coupon is applicable to all users
        allowed_users: Vec<String>,
        /// Whether or not the coupon is usable or not
        usable: bool,
        /// The target types the coupon can be applied to
        ///
        /// If empty, the coupon is applicable to all target types
        target_types: Vec<String>,
    },
    /// Deletes a shop coupon
    Delete {
        /// The ID of the shop coupon
        id: String,
    },
}

use super::types::staff_members::StaffMemberAction;
use crate::impls::target_types::TargetType;
use crate::panelapi::types::staff_positions::StaffPositionAction;
use crate::panelapi::types::{
    auth::AuthorizeAction,
    blog::BlogAction,
    bot_whitelist::BotWhitelistAction,
    cdn::CdnAssetAction,
    changelogs::ChangelogAction,
    partners::PartnerAction,
    shop_items::{ShopCouponAction, ShopItemAction, ShopItemBenefitAction},
    staff_disciplinary::StaffDisciplinaryTypeAction,
    vote_credit_tiers::VoteCreditTierAction,
};
use crate::rpc::core::RPCMethod;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, TS, Display, Clone, EnumVariantNames)]
#[ts(export, export_to = ".generated/PanelQuery.ts")]
pub enum PanelQuery {
    /// Authorization-related commands
    Authorize {
        /// Authorize protocol version, should be `AUTH_VERSION`
        version: u16,
        /// Action to take
        action: AuthorizeAction,
    },
    /// Returns configuration data for the panel
    Hello {
        /// Login token
        login_token: String,
        /// Hello protocol version, should be `HELLO_VERSION`
        version: u16,
    },
    /// Returns base analytics
    BaseAnalytics {
        /// Login token
        login_token: String,
    },
    /// Returns user information given a user id, returning a dovewing PartialUser
    GetUser {
        /// Login token
        login_token: String,
        /// User ID to fetch details for
        user_id: String,
    },
    /// Returns the bot queue
    ///
    /// This is public to all staff members
    BotQueue {
        /// Login token
        login_token: String,
    },
    /// Executes an RPC on a target
    ///
    /// The endpoint itself is public to all staff members however RPC will only execute if the user has permission for the RPC method
    ExecuteRpc {
        /// Login token
        login_token: String,
        /// Target Type
        target_type: TargetType,
        /// RPC Method
        method: RPCMethod,
    },
    /// Returns all RPC actions available
    ///
    /// Setting filtered will filter RPC actions to that what the user has access to
    ///
    /// This is public to all staff members
    GetRpcMethods {
        /// Login token
        login_token: String,
        /// Filtered
        filtered: bool,
    },
    /// Gets the list of all RPC log entries made
    GetRpcLogEntries {
        /// Login token
        login_token: String,
    },
    /// Searches for a bot based on a query
    ///
    /// This is public to all staff members
    SearchEntitys {
        /// Login token
        login_token: String,
        /// Target type
        target_type: TargetType,
        /// Query
        query: String,
    },
    /// Uploads a chunk of data returning a chunk ID
    ///
    /// Chunks expire after 10 minutes and are stored in memory
    ///
    /// After uploading all chunks for a file, use `AddFile` to create the file
    ///
    /// Needs `cdn.upload_chunk` permission
    UploadCdnFileChunk {
        /// Login token
        login_token: String,
        /// Array of bytes of the chunk contents
        chunk: Vec<u8>,
    },
    /// Lists all available CDN scopes
    ///
    /// Needs `cdn.list_scopes` permission
    ListCdnScopes {
        /// Login token
        login_token: String,
    },
    /// Returns the main CDN scope for Infinity List
    ///
    /// This is public to all staff members
    GetMainCdnScope {
        /// Login token
        login_token: String,
    },
    /// Updates/handles an asset on the CDN
    ///
    /// Needs `cdn.update_asset` permission. Not yet granular/action specific
    UpdateCdnAsset {
        /// Login token
        login_token: String,
        /// CDN scope
        ///
        /// This describes a location where the CDN may be stored on disk and should be a full path to it
        ///
        /// Currently the panel uses the following scopes:
        ///
        /// `ibl@main`
        cdn_scope: String,
        /// Asset name
        name: String,
        /// Path
        path: String,
        /// Action to take
        action: CdnAssetAction,
    },
    /// Updates/handles partners
    UpdatePartners {
        /// Login token
        login_token: String,
        /// Action
        action: PartnerAction,
    },
    /// Updates/handles the changelog of the list
    UpdateChangelog {
        /// Login token
        login_token: String,
        /// Action
        action: ChangelogAction,
    },
    /// Updates/handles the blog of the list
    UpdateBlog {
        /// Login token
        login_token: String,
        /// Action
        action: BlogAction,
    },
    /// Fetch and modify staff positions
    UpdateStaffPositions {
        /// Login token
        login_token: String,
        /// Action
        action: StaffPositionAction,
    },
    /// Fetch and modify staff members
    UpdateStaffMembers {
        /// Login token
        login_token: String,
        /// Action
        action: StaffMemberAction,
    },
    /// Fetch and update staff disciplinary types
    UpdateStaffDisciplinaryType {
        /// Login token
        login_token: String,
        /// Action
        action: StaffDisciplinaryTypeAction,
    },
    /// Fetch and update/modify vote credit tiers
    UpdateVoteCreditTiers {
        /// Login token
        login_token: String,
        /// Action
        action: VoteCreditTierAction,
    },
    /// Fetch and update/modify shop items
    UpdateShopItems {
        /// Login token
        login_token: String,
        /// Action
        action: ShopItemAction,
    },
    /// Fetch and update/modify shop item benefits
    UpdateShopItemBenefits {
        /// Login token
        login_token: String,
        /// Action
        action: ShopItemBenefitAction,
    },
    /// Fetch and update/modify shop coupons
    UpdateShopCoupons {
        /// Login token
        login_token: String,
        /// Action
        action: ShopCouponAction,
    },
    /// Fetch and update/modify bot whitelist
    UpdateBotWhitelist {
        /// Login token
        login_token: String,
        /// Action
        action: BotWhitelistAction,
    },
    /// Create a request to a/an Popplio staff endpoint
    PopplioStaff {
        /// Login token
        login_token: String,
        /// Path
        path: String,
        /// Method
        method: String,
        /// Body
        body: String,
    },
}

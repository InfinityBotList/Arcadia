use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/BotWhitelist.ts")]
pub struct BotWhitelist {
    /// The Bot's ID
    pub bot_id: String,
    /// The user id who added the bot to the whitelist
    pub user_id: String,
    /// The reason
    pub reason: String,
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
#[ts(export, export_to = ".generated/BotWhitelistAction.ts")]
pub enum BotWhitelistAction {
    /// List all currently whitelisted bots
    #[default]
    List,
    /// Create a new bot whitelist entry
    Add {
        /// The ID of the bot
        bot_id: String,
        /// The reason
        reason: String,
    },
    /// Edit a bot whitelist entry
    Edit {
        /// The ID of the bot
        bot_id: String,
        /// The reason
        reason: String,
    },
    /// Delete a bot whitelist entry
    Delete {
        /// The ID of the bot
        bot_id: String,
    },
}

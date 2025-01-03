use log::error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{CreateEmbed, CreateEmbedFooter, CreateMessage, GuildId, UserId};
use serenity::model::Color;
use sqlx::{types::Uuid, PgPool};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;

use crate::{
    impls::{target_types::TargetType, utils::get_user_perms},
    Error,
};
use kittycat::perms;
use utoipa::ToSchema;

/// Helper function to check if a member is on a server, returning a boolean
pub fn member_on_guild(
    cache_http: &botox::cache::CacheHttpImpl,
    guild_id: GuildId,
    user_id: UserId,
) -> bool {
    if let Some(guild) = cache_http.cache.guild(guild_id) {
        guild.members.get(&user_id).is_some()
    } else {
        false
    }
}

#[derive(Serialize, Deserialize, ToSchema, TS, EnumString, EnumVariantNames, Display, Clone)]
#[ts(export, export_to = ".generated/RPCMethod.ts")]
pub enum RPCMethod {
    Claim {
        target_id: String,
        force: bool,
    },
    Unclaim {
        target_id: String,
        reason: String,
    },
    Approve {
        target_id: String,
        reason: String,
    },
    Deny {
        target_id: String,
        reason: String,
    },
    Unverify {
        target_id: String,
        reason: String,
    },
    PremiumAdd {
        target_id: String,
        reason: String,
        time_period_hours: i32,
    },
    PremiumRemove {
        target_id: String,
        reason: String,
    },
    VoteBanAdd {
        target_id: String,
        reason: String,
    },
    VoteBanRemove {
        target_id: String,
        reason: String,
    },
    VoteReset {
        target_id: String,
        reason: String,
    },
    VoteResetAll {
        reason: String,
    },
    ForceRemove {
        target_id: String,
        reason: String,
        kick: bool,
    },
    CertifyAdd {
        target_id: String,
        reason: String,
    },
    CertifyRemove {
        target_id: String,
        reason: String,
    },
    BotTransferOwnershipUser {
        target_id: String,
        reason: String,
        new_owner: String,
    },
    BotTransferOwnershipTeam {
        target_id: String,
        reason: String,
        new_team: String,
    },
    AppBanUser {
        target_id: String,
        reason: String,
    },
    AppUnbanUser {
        target_id: String,
        reason: String,
    },
}

impl Default for RPCMethod {
    fn default() -> Self {
        RPCMethod::Claim {
            target_id: "bot_id".to_string(),
            force: false,
        }
    }
}

pub enum RPCSuccess {
    NoContent,
    Content(String),
}

impl RPCSuccess {
    pub fn content(&self) -> Option<&str> {
        match self {
            RPCSuccess::Content(c) => Some(c),
            _ => None,
        }
    }
}

/// Represents a single RPC field
#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = ".generated/RPCField.ts")]
pub struct RPCField {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    pub icon: String,
    pub placeholder: String,
}

impl RPCField {
    fn target_id() -> Self {
        RPCField {
            id: "target_id".to_string(),
            label: "Target ID".to_string(),
            field_type: FieldType::Text,
            icon: "ic:twotone-access-time-filled".to_string(),
            placeholder: "The Target ID to perform the action on".to_string(),
        }
    }

    fn reason() -> Self {
        RPCField {
            id: "reason".to_string(),
            label: "Reason".to_string(),
            field_type: FieldType::Textarea,
            icon: "material-symbols:question-mark".to_string(),
            placeholder: "Reason for performing this action".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = ".generated/RPCFieldType.ts")]
// Allow dead code
#[allow(dead_code)]
/// Represents a field type
pub enum FieldType {
    Text,
    Textarea,
    Number,
    Hour, // Time expressed as a number of hours
    Boolean,
}

pub struct RPCHandle {
    pub pool: PgPool,
    pub cache_http: botox::cache::CacheHttpImpl,
    pub user_id: String,
    pub target_type: TargetType,
}

impl RPCMethod {
    pub fn supported_target_types(&self) -> Vec<TargetType> {
        match self {
            RPCMethod::Claim { .. } => vec![TargetType::Bot],
            RPCMethod::Unclaim { .. } => vec![TargetType::Bot],
            RPCMethod::Approve { .. } => vec![TargetType::Bot],
            RPCMethod::Deny { .. } => vec![TargetType::Bot],
            RPCMethod::Unverify { .. } => vec![TargetType::Bot],
            RPCMethod::PremiumAdd { .. } => vec![TargetType::Bot],
            RPCMethod::PremiumRemove { .. } => vec![TargetType::Bot],
            RPCMethod::VoteBanAdd { .. } => vec![TargetType::Bot],
            RPCMethod::VoteBanRemove { .. } => vec![TargetType::Bot],
            RPCMethod::VoteReset { .. } => vec![
                TargetType::Bot,
                TargetType::Server,
                TargetType::Team,
                TargetType::Pack,
            ],
            RPCMethod::VoteResetAll { .. } => vec![
                TargetType::Bot,
                TargetType::Server,
                TargetType::Team,
                TargetType::Pack,
            ],
            RPCMethod::ForceRemove { .. } => vec![TargetType::Bot],
            RPCMethod::CertifyAdd { .. } => vec![TargetType::Bot],
            RPCMethod::CertifyRemove { .. } => vec![TargetType::Bot],
            RPCMethod::BotTransferOwnershipUser { .. } => vec![TargetType::Bot],
            RPCMethod::BotTransferOwnershipTeam { .. } => vec![TargetType::Bot],
            RPCMethod::AppBanUser { .. } => vec![TargetType::User],
            RPCMethod::AppUnbanUser { .. } => vec![TargetType::User],
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Claim { .. } => {
                "Claim a entity. Be sure to claim entities that you are going to review!"
            }
            Self::Unclaim { .. } => {
                "Unclaim a entity. Be sure to use this if you can't review the entity!"
            }
            Self::Approve { .. } => "Approve a entity. Needs to be claimed first.",
            Self::Deny { .. } => "Deny a entity. Needs to be claimed first.",
            Self::Unverify { .. } => "Unverifies a bot on the list",
            Self::PremiumAdd { .. } => "Adds premium to a bot for a given time period",
            Self::PremiumRemove { .. } => "Removes premium from a bot",
            Self::VoteBanAdd { .. } => "Vote-bans the bot in question",
            Self::VoteBanRemove { .. } => "Removes the vote-ban from the bot in question",
            Self::VoteReset { .. } => "Reset the votes of a given entity (bot/pack/server etc.",
            Self::VoteResetAll { .. } => "Reset the votes of a given entity (bot/pack/server etc.",
            Self::ForceRemove { .. } => "Forcefully removes a bot from the list",
            Self::CertifyAdd { .. } => {
                "Certifies a entity. Recommended to use apps instead however"
            }
            Self::CertifyRemove { .. } => "Uncertifies a bot",
            Self::BotTransferOwnershipUser { .. } => {
                "Transfers the ownership of a bot to a new user"
            }
            Self::BotTransferOwnershipTeam { .. } => {
                "Transfers the ownership of a bot to a new team"
            }
            Self::AppBanUser { .. } => "Ban user from apps",
            Self::AppUnbanUser { .. } => "Unban user from apps",
        }
        .to_string()
    }

    pub fn label(&self) -> String {
        match self {
            Self::Claim { .. } => "Claim entity",
            Self::Unclaim { .. } => "Unclaim entity",
            Self::Approve { .. } => "Approve entity",
            Self::Deny { .. } => "Deny entity",
            Self::Unverify { .. } => "Unverify entity",
            Self::PremiumAdd { .. } => "Add Premium",
            Self::PremiumRemove { .. } => "Remove Premium",
            Self::VoteBanAdd { .. } => "Vote Ban",
            Self::VoteBanRemove { .. } => "Unvote Ban",
            Self::VoteReset { .. } => "Vote Reset Entity",
            Self::VoteResetAll { .. } => "Vote Reset All Entities",
            Self::ForceRemove { .. } => "Force Remove",
            Self::CertifyAdd { .. } => "Certify",
            Self::CertifyRemove { .. } => "Uncertify",
            Self::BotTransferOwnershipUser { .. } => "Set Bot Owner [User]",
            Self::BotTransferOwnershipTeam { .. } => "Set Bot Owner [Team]",
            Self::AppBanUser { .. } => "Ban from apps [User]",
            Self::AppUnbanUser { .. } => "Unban from apps [User]",
        }
        .to_string()
    }

    pub async fn handle(&self, state: RPCHandle) -> Result<RPCSuccess, Error> {
        // First ensure that target type on handle is in supported target types
        if !self.supported_target_types().contains(&state.target_type) {
            return Err("This method does not support this target type yet".into());
        }

        // Next, ensure we have the permissions needed
        let user_perms = get_user_perms(&state.pool, &state.user_id).await?.resolve();

        let required_perm = format!("rpc.{}", self).into();
        if !perms::has_perm(&user_perms, &required_perm) {
            return Err(format!(
                "You need {} permission to use {}",
                required_perm,
                &self.to_string()
            )
            .into());
        }

        // Also ensure that onboarding has happened
        if sqlx::query!(
            "SELECT COUNT(*) FROM staff_onboardings WHERE user_id = $1 AND void = false AND state = 'completed' AND NOW() - created_at < INTERVAL '1 month'",
            &state.user_id,
        )
        .fetch_one(&state.pool)
        .await?
        .count
        .unwrap_or(0) == 0 {
            return Err("You need to have completed onboarding in order to use RPC!".into());
        }

        // Insert into rpc_logs
        let id = sqlx::query!(
            "INSERT INTO rpc_logs (method, user_id, data) VALUES ($1, $2, $3) RETURNING id",
            self.to_string(),
            &state.user_id,
            json!(self)
        )
        .fetch_one(&state.pool)
        .await?;

        // Get number of requests in the last 7 minutes
        let res = sqlx::query!(
            "SELECT COUNT(*) FROM rpc_logs WHERE user_id = $1 AND NOW() - created_at < INTERVAL '7 minutes'",
            &state.user_id
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|_| "Failed to get ratelimit count")?;

        let count = res.count.unwrap_or_default();

        if count > 5 {
            sqlx::query!(
                "DELETE FROM staffpanel__authchain WHERE user_id = $1",
                &state.user_id,
            )
            .execute(&state.pool)
            .await
            .map_err(|_| "Failed to reset user token")?;

            return Err("Rate limit exceeded. Wait 5-10 minutes and try again?".into());
        }

        // Now we can handle the method
        let resp = self.handle_method(&state).await;

        if resp.is_ok() {
            sqlx::query!(
                "UPDATE rpc_logs SET state = $1 WHERE id = $2",
                "success",
                id.id
            )
            .execute(&state.pool)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE rpc_logs SET state = $1 WHERE id = $2",
                resp.as_ref()
                    .err()
                    .ok_or("Err variant doesnt have an error!")?
                    .to_string(),
                id.id
            )
            .execute(&state.pool)
            .await?;
        }

        resp
    }

    /// The low-level method handler
    async fn handle_method(&self, state: &RPCHandle) -> Result<RPCSuccess, Error> {
        match self {
            RPCMethod::Claim { target_id, force } => {
                // Check if its claimed by someone
                let claimed = sqlx::query!(
                    "SELECT type, claimed_by FROM bots WHERE bot_id = $1",
                    target_id
                )
                .fetch_one(&state.pool)
                .await?;

                if claimed.r#type != "pending" {
                    return Err("This bot is not pending review".into());
                }

                if claimed.r#type == "testbot" {
                    return Err("This bot is a test bot".into());
                }

                if !force {
                    if let Some(claimed_by) = claimed.claimed_by {
                        return Err(
                            format!("This bot is already claimed by <@{}>", claimed_by).into()
                        );
                    }
                }

                let owners = crate::impls::utils::get_entity_managers(
                    TargetType::Bot,
                    target_id,
                    &state.pool,
                )
                .await?;

                // Claim it
                sqlx::query!(
                    "UPDATE bots SET last_claimed = NOW(), claimed_by = $1 WHERE bot_id = $2",
                    &state.user_id,
                    target_id
                )
                .execute(&state.pool)
                .await?;

                sqlx::query!(
                    "INSERT INTO staff_general_logs (user_id, action, data) VALUES ($1, $2, $3)",
                    &state.user_id,
                    "claimed",
                    json!({
                        "target_id": target_id,
                        "claimed_by_prev": claimed.claimed_by,
                    })
                )
                .execute(&state.pool)
                .await?;

                // Send a message to the bot owner
                let msg = CreateMessage::default()
                    .content(owners.mention_users())
                    .embed(
                        CreateEmbed::default()
                            .title(" Claimed!")
                            .description(format!(
                                "<@{}> has claimed <@{}>",
                                &state.user_id, target_id
                            ))
                            .color(Color::BLURPLE)
                            .field("Force Claim", force.to_string(), false)
                            .footer(CreateEmbedFooter::new(
                                "This is completely normal, don't worry!",
                            )),
                    );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::Unclaim { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Check if its claimed by someone
                let claimed = sqlx::query!(
                    "SELECT type, claimed_by, owner FROM bots WHERE bot_id = $1",
                    target_id
                )
                .fetch_one(&state.pool)
                .await?;

                if claimed.r#type == "testbot" {
                    return Err("This bot is a test bot".into());
                }

                if claimed.r#type != "pending" {
                    return Err("This bot is not pending review".into());
                }

                let owners = crate::impls::utils::get_entity_managers(
                    TargetType::Bot,
                    target_id,
                    &state.pool,
                )
                .await?;

                if claimed.claimed_by.is_none() {
                    return Err(format!("<@{}> is not claimed", target_id).into());
                }

                sqlx::query!(
                    "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                sqlx::query!(
                    "INSERT INTO staff_general_logs (user_id, action, data) VALUES ($1, $2, $3)",
                    &state.user_id,
                    "unclaimed",
                    json!({
                        "target_id": target_id,
                        "claimed_by_prev": claimed.claimed_by,
                    })
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().content(owners.mention_users()).embed(
                    CreateEmbed::new()
                        .title(" Unclaimed!")
                        .description(format!(
                            "<@{}> has unclaimed <@{}>",
                            &state.user_id, target_id
                        ))
                        .field("Reason", reason, false)
                        .footer(CreateEmbedFooter::new(
                            "This is completely normal, don't worry!",
                        )),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::Approve { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                let claimed = sqlx::query!(
                    "SELECT type, claimed_by, last_claimed FROM bots WHERE bot_id = $1",
                    target_id
                )
                .fetch_one(&state.pool)
                .await?;

                if claimed.r#type != "pending" {
                    return Err("Entity is not pending review?".into());
                }

                if claimed.claimed_by.is_none()
                    || claimed.claimed_by.as_ref().unwrap().is_empty()
                    || claimed.last_claimed.is_none()
                {
                    return Err(format!(
                        "<@{}> is not claimed? Do ``/claim`` to claim this bot first!",
                        target_id
                    )
                    .into());
                }

                let owners = crate::impls::utils::get_entity_managers(
                    TargetType::Bot,
                    target_id,
                    &state.pool,
                )
                .await?;

                let mut tx = state.pool.begin().await?;

                sqlx::query!(
                    "UPDATE bots SET type = 'approved', claimed_by = NULL WHERE bot_id = $1",
                    target_id
                )
                .execute(&mut *tx)
                .await?;

                // Add to cache server using borealis
                #[derive(serde::Serialize, serde::Deserialize)]
                struct BorealisCacheServer {
                    guild_id: String,
                    name: String,
                    invite_code: String,
                    added: bool,
                }

                let csr = reqwest::Client::new()
                    .post(format!(
                        "{}/addBotToCacheServer?bot_id={}&ignore_bot_type=true",
                        crate::config::CONFIG.borealis_url,
                        target_id
                    ))
                    .send()
                    .await?
                    .json::<BorealisCacheServer>()
                    .await
                    .map_err(|e| format!("Error decoding borealis response: {:?}", e))?;

                let msg = CreateMessage::default()
                    .content(owners.mention_users())
                    .embed(
                        CreateEmbed::default()
                            .title(" Approved!")
                            .url(format!(
                                "{}/bots/{}",
                                crate::config::CONFIG.frontend_url.get(),
                                target_id
                            ))
                            .description(format!(
                                "<@!{}> has approved <@!{}>",
                                &state.user_id, target_id
                            ))
                            .field(
                                "Cache Server",
                                format!("[{}](https://discord.gg/{})", csr.name, csr.invite_code),
                                true,
                            )
                            .field("Feedback", reason, true)
                            .field("Moderator", "<@!".to_string() + &state.user_id + ">", true)
                            .footer(CreateEmbedFooter::new("Well done, young traveller!"))
                            .color(0x00ff00),
                    );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                tx.commit().await?;

                let owners = crate::impls::utils::get_entity_managers(
                    TargetType::Bot,
                    target_id,
                    &state.pool,
                )
                .await?
                .all();

                for owner in owners {
                    let owner_snow = owner.parse::<UserId>()?;

                    if member_on_guild(
                        &state.cache_http,
                        crate::config::CONFIG.servers.main,
                        owner_snow,
                    ) {
                        // Add role to user
                        if let Err(e) = state
                            .cache_http
                            .http
                            .add_member_role(
                                crate::config::CONFIG.servers.main,
                                owner_snow,
                                crate::config::CONFIG.roles.bot_developer,
                                Some("Autorole due to bots owned"),
                            )
                            .await
                        {
                            error!("Failed to add role to user: {}", e);
                        }
                    }
                }

                // Kick the bot from the testing server
                if member_on_guild(
                    &state.cache_http,
                    crate::config::CONFIG.servers.testing,
                    target_id.parse()?,
                ) {
                    if let Err(e) = state
                        .cache_http
                        .http
                        .kick_member(
                            crate::config::CONFIG.servers.testing,
                            target_id.parse()?,
                            Some("Bot approved"),
                        )
                        .await
                    {
                        error!("Failed to kick bot from testing server: {}", e);
                    }
                }

                let invite_data =
                    sqlx::query!("SELECT client_id FROM bots WHERE bot_id = $1", target_id)
                        .fetch_one(&state.pool)
                        .await?;

                Ok(
                    RPCSuccess::Content(
                        format!(
                            "**Cache Server Invite:** {csr_invite}\n**Invite URL:** https://discord.com/api/v10/oauth2/authorize?client_id={client_id}&permissions=0&scope=bot%20applications.commands&guild_id={guild_id}", 
                            csr_invite = "https://discord.gg/".to_string() + &csr.invite_code,
                            client_id = invite_data.client_id,
                            guild_id = csr.guild_id
                        )
                    )
                )
            }
            RPCMethod::Deny { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                let claimed = sqlx::query!(
                    "SELECT type, claimed_by, owner, last_claimed FROM bots WHERE bot_id = $1",
                    target_id
                )
                .fetch_one(&state.pool)
                .await?;

                if claimed.r#type != "pending" {
                    return Err(" is not pending review?".into());
                }

                if claimed.claimed_by.is_none()
                    || claimed.claimed_by.as_ref().unwrap().is_empty()
                    || claimed.last_claimed.is_none()
                {
                    return Err(format!(
                        "<@{}> is not claimed? Do ``/claim`` to claim this bot first!",
                        target_id
                    )
                    .into());
                }

                let owners = crate::impls::utils::get_entity_managers(
                    TargetType::Bot,
                    target_id,
                    &state.pool,
                )
                .await?;

                sqlx::query!(
                    "UPDATE bots SET type = 'denied', claimed_by = NULL WHERE bot_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().content(owners.mention_users()).embed(
                    CreateEmbed::default()
                        .title(" Denied!")
                        .url(format!(
                            "{}/bots/{}",
                            crate::config::CONFIG.frontend_url.get(),
                            target_id
                        ))
                        .description(format!("<@{}> has denied <@{}>", &state.user_id, target_id))
                        .field("Reason", reason, true)
                        .field("Moderator", "<@!".to_string() + &state.user_id + ">", true)
                        .footer(CreateEmbedFooter::new(
                            "Well done, young traveller at getting denied from the club!",
                        ))
                        .color(0x00ff00),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::Unverify { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                let bot_type_rec =
                    sqlx::query!("SELECT type FROM bots WHERE bot_id = $1", target_id)
                        .fetch_one(&state.pool)
                        .await?;

                if bot_type_rec.r#type == "certified" {
                    return Err("Certified bots cannot be unverified".into());
                }

                sqlx::query!(
                    "UPDATE bots SET type = 'pending', claimed_by = NULL WHERE bot_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::default().embed(
                    CreateEmbed::default()
                        .title("__ Unverified For Futher Review!__")
                        .field("Reason", reason, true)
                        .field("Moderator", "<@".to_string() + &state.user_id + ">", true)
                        .field("", "<@!".to_string() + target_id + ">", true)
                        .footer(CreateEmbedFooter::new("Gonna be pending further review..."))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::PremiumAdd {
                target_id,
                reason,
                time_period_hours,
            } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                // Set premium_period_length which is a postgres interval
                sqlx::query!(
                    "UPDATE bots SET start_premium_period = NOW(), premium_period_length = make_interval(hours => $1), premium = true WHERE bot_id = $2",
                    time_period_hours,
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Premium Added!")
                        .description(format!(
                            "<@{}> has added premium to <@{}> for {} hours",
                            &state.user_id, target_id, time_period_hours
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Well done, young traveller! Use it wisely...",
                        ))
                        .color(0x00ff00),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::PremiumRemove { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                // Set premium_period_length which is a postgres interval
                sqlx::query!(
                    "UPDATE bots SET premium = false WHERE bot_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Premium Removed!")
                        .description(format!(
                            "<@{}> has removed premium from <@{}>",
                            state.user_id, target_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Well done, young traveller. Sad to see you go...",
                        ))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::VoteBanAdd { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET vote_banned = true WHERE bot_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Vote Ban Edit!")
                        .description(format!(
                            "<@{}> has set the vote ban on <@{}>",
                            state.user_id, target_id,
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Remember: don't abuse our services!",
                        ))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::VoteBanRemove { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET vote_banned = false WHERE bot_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Vote Ban Removed!")
                        .description(format!(
                            "<@{}> has removed the vote ban on <@{}>",
                            state.user_id, target_id,
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Remember: don't abuse our services!",
                        ))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::VoteReset { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                sqlx::query!("UPDATE entity_votes SET void = TRUE, void_reason = 'Votes (single entity) reset', voided_at = NOW() WHERE target_type = $1 AND target_id = $2 AND void = FALSE", state.target_type.to_string(), target_id)
                    .execute(&state.pool)
                    .await?;

                let msg = CreateMessage::default().embed(
                    CreateEmbed::default()
                        .title("__Entity Vote Reset!__")
                        .field("Reason", reason, true)
                        .field("Moderator", "<@".to_string() + &state.user_id + ">", true)
                        .field("Target ID", target_id, true)
                        .field("Target Type", state.target_type.to_string(), true)
                        .footer(CreateEmbedFooter::new("Sad life :("))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::VoteResetAll { reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                let mut tx = state.pool.begin().await?;

                sqlx::query!("UPDATE entity_votes SET void = TRUE, void_reason = 'Votes (all entities) reset', voided_at = NOW() WHERE target_type = $1 AND immutable = false", state.target_type.to_string())
                    .execute(&mut *tx)
                    .await?;

                tx.commit().await?;

                let msg = CreateMessage::default().embed(
                    CreateEmbed::default()
                        .title("__All Entity Votes Reset!__")
                        .field("Reason", reason, true)
                        .field("Moderator", "<@".to_string() + &state.user_id + ">", true)
                        .field("Target Type", state.target_type.to_string(), true)
                        .footer(CreateEmbedFooter::new("Sad life :("))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::ForceRemove {
                target_id,
                reason,
                kick,
            } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                let target_id_snow = target_id.parse::<UserId>()?;

                if crate::config::CONFIG
                    .protected_bots
                    .contains(&target_id_snow)
                    && *kick
                {
                    return Err("You can't force delete this bot with 'kick' enabled!".into());
                }

                sqlx::query!("DELETE FROM bots WHERE bot_id = $1", target_id)
                    .execute(&state.pool)
                    .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title(" Force Deleted!")
                        .description(format!(
                            "<@{}> has force-removed <@{}> for violating our rules or Discord ToS",
                            state.user_id, target_id,
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Remember: don't abuse our services!",
                        ))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                if *kick {
                    // Check that the bot is in the server
                    let bot_in_server = member_on_guild(
                        &state.cache_http,
                        crate::config::CONFIG.servers.main,
                        target_id_snow,
                    );

                    if bot_in_server {
                        state
                            .cache_http
                            .http
                            .kick_member(
                                crate::config::CONFIG.servers.main,
                                target_id_snow,
                                Some("Force deleted via RPC with kick set to true"),
                            )
                            .await?;
                    }
                }

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::CertifyAdd { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET type = 'certified' WHERE bot_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title(" Force Certified!")
                        .description(format!(
                            "<@{}> has force-certified <@{}>",
                            state.user_id, target_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new("Neat"))
                        .color(0xff0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::CertifyRemove { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET type = 'approved' WHERE bot_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title(" Uncertified!")
                        .description(format!(
                            "<@{}> has uncertified <@{}>",
                            state.user_id, target_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Uh oh, looks like you've been naughty...",
                        ))
                        .color(0xff0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotTransferOwnershipUser {
                target_id,
                new_owner,
                reason,
            } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                // Check that the bot is not in a team
                let team_owner =
                    sqlx::query!("SELECT team_owner FROM bots WHERE bot_id = $1", target_id)
                        .fetch_one(&state.pool)
                        .await?;

                if team_owner.team_owner.is_some() {
                    return Err(" is in a team. Please use BotTransferOwnershipTeam".into());
                }

                sqlx::query!(
                    "UPDATE bots SET owner = $2 WHERE bot_id = $1",
                    target_id,
                    new_owner
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title(" Ownership Force Update!")
                        .description(format!(
                            "<@{}> has force-updated the ownership of <@{}> to <@{}>",
                            state.user_id, target_id, new_owner
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Contact support if you think this is a mistake",
                        ))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotTransferOwnershipTeam {
                target_id,
                new_team,
                reason,
            } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                // Parse the team ID
                let team_id = match new_team.parse::<Uuid>() {
                    Ok(id) => id,
                    Err(_) => return Err("Invalid team ID".into()),
                };

                // Check that the bot is not in a team
                let team_owner =
                    sqlx::query!("SELECT team_owner FROM bots WHERE bot_id = $1", target_id)
                        .fetch_one(&state.pool)
                        .await?;

                if team_owner.team_owner.is_none() {
                    return Err(" is not in a team. Please use TransferOwnership".into());
                }

                sqlx::query!(
                    "UPDATE bots SET team_owner = $2 WHERE bot_id = $1",
                    target_id,
                    team_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title(" Ownership Force Update!")
                        .description(format!(
                            "<@{}> has force-updated the ownership of <@{}> to team {}",
                            state.user_id, target_id, team_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Contact support if you think this is a mistake",
                        ))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::AppBanUser { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the user actually exists
                let user = sqlx::query!("SELECT COUNT(*) FROM users WHERE user_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if user.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                // Set app_banned to true
                sqlx::query!(
                    "UPDATE users SET app_banned = true WHERE user_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("[Apps] Banned User")
                        .description(format!(
                            "<@{}> has banned <@{}> from using apps.",
                            state.user_id, target_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Well done, young traveller. Sad to see you go...",
                        ))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::AppUnbanUser { target_id, reason } => {
                if reason.len() > 2000 {
                    return Err("Reason must be lower than/equal to 2000 characters".into());
                }

                // Ensure the user actually exists
                let user = sqlx::query!("SELECT COUNT(*) FROM users WHERE user_id = $1", target_id)
                    .fetch_one(&state.pool)
                    .await?;

                if user.count.unwrap_or_default() == 0 {
                    return Err(" does not exist".into());
                }

                // Set app_banned to false
                sqlx::query!(
                    "UPDATE users SET app_banned = false WHERE user_id = $1",
                    target_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("[Apps] Unbanned User")
                        .description(format!(
                            "<@{}> has unbanned <@{}> from using apps.",
                            state.user_id, target_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new("Welcome, back!"))
                        .color(0xFF0000),
                );

                crate::config::CONFIG
                    .channels
                    .mod_logs
                    .send_message(&state.cache_http.http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
        }
    }

    // Returns a set of RPCField's for a given enum variant
    pub fn method_fields(&self) -> Vec<RPCField> {
        match self {
            RPCMethod::Claim { .. } => vec![
                RPCField::target_id(),
                RPCField {
                    id: "force".to_string(),
                    label: "Force claim bot?".to_string(),
                    field_type: FieldType::Boolean,
                    icon: "fa-solid:sign-out-alt".to_string(),
                    placeholder: "Yes/No".to_string(),
                },
            ],
            RPCMethod::Unclaim { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::Approve { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::Deny { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::Unverify { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::PremiumAdd { .. } => vec![
                RPCField::target_id(),
                RPCField {
                    id: "time_period_hours".to_string(),
                    label: "Time [X unit(s)]".to_string(),
                    field_type: FieldType::Hour,
                    icon: "material-symbols:timer".to_string(),
                    placeholder: "Time period. Format: X years/days/hours".to_string(),
                },
                RPCField::reason(),
            ],
            RPCMethod::PremiumRemove { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::VoteBanAdd { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::VoteBanRemove { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::VoteReset { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::VoteResetAll { .. } => vec![RPCField::reason()],
            RPCMethod::ForceRemove { .. } => vec![
                RPCField::target_id(),
                RPCField {
                    id: "kick".to_string(),
                    label: "Kick the bot from the server".to_string(),
                    field_type: FieldType::Boolean,
                    icon: "fa-solid:sign-out-alt".to_string(),
                    placeholder: "Kick the bot from the server".to_string(),
                },
                RPCField::reason(),
            ],
            RPCMethod::CertifyAdd { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::CertifyRemove { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::BotTransferOwnershipUser { .. } => vec![
                RPCField::target_id(),
                RPCField {
                    id: "new_owner".to_string(),
                    label: "User ID".to_string(),
                    field_type: FieldType::Text,
                    icon: "material-symbols:timer".to_string(),
                    placeholder: "New Owner".to_string(),
                },
                RPCField::reason(),
            ],
            RPCMethod::BotTransferOwnershipTeam { .. } => vec![
                RPCField::target_id(),
                RPCField {
                    id: "new_team".to_string(),
                    label: "Team ID".to_string(),
                    field_type: FieldType::Text,
                    icon: "material-symbols:timer".to_string(),
                    placeholder: "New Team".to_string(),
                },
                RPCField::reason(),
            ],
            RPCMethod::AppBanUser { .. } => vec![RPCField::target_id(), RPCField::reason()],
            RPCMethod::AppUnbanUser { .. } => vec![RPCField::target_id(), RPCField::reason()],
        }
    }
}

use std::num::NonZeroU64;

use log::error;
use poise::serenity_prelude::{
    ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, GuildId, RoleId, UserId,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, types::Uuid};
use strum_macros::{Display, EnumString, EnumVariantNames};
use ts_rs::TS;

use crate::{impls, Error};

#[derive(Deserialize, TS)]
#[ts(export, export_to = ".generated/RPCRequest.ts")]
pub struct RPCRequest {
    pub user_id: String,
    pub token: String,
    pub method: RPCMethod,
    pub protocol: u8,
}

#[derive(Serialize, Deserialize, TS, EnumString, EnumVariantNames, Display)]
#[ts(export, export_to = ".generated/RPCMethod.ts")]
#[allow(clippy::enum_variant_names)]
pub enum RPCMethod {
    BotApprove {
        bot_id: String,
        reason: String,
    },
    BotDeny {
        bot_id: String,
        reason: String,
    },
    BotVoteReset {
        bot_id: String,
        reason: String,
    },
    BotVoteResetAll {
        reason: String,
    },
    BotUnverify {
        bot_id: String,
        reason: String,
    },
    BotPremiumAdd {
        bot_id: String,
        reason: String,
        time_period_hours: i32,
    },
    BotPremiumRemove {
        bot_id: String,
        reason: String,
    },
    BotVoteBanAdd {
        bot_id: String,
        reason: String,
    },
    BotVoteBanRemove {
        bot_id: String,
        reason: String,
    },
    BotForceRemove {
        bot_id: String,
        reason: String,
        kick: bool,
    },
    BotCertifyAdd {
        bot_id: String,
        reason: String,
    },
    BotCertifyRemove {
        bot_id: String,
        reason: String,
    },
    BotVoteCountSet {
        bot_id: String,
        count: i32,
        reason: String,
    },
    BotTransferOwnership {
        bot_id: String,
        new_owner: String,
        reason: String,
    },
    BotTransferOwnershipTeam {
        bot_id: String,
        new_team: String,
        reason: String,
    },
}

pub struct RPCHandle {
    pub pool: PgPool,
    pub cache_http: impls::cache::CacheHttpImpl,
    pub user_id: String,
}

pub enum RPCPerms {
    Owner,
    Head,  // Either hadmin/hdev
    Admin, //admin
    Staff,
}

impl RPCMethod {
    fn needs_perms(&self) -> RPCPerms {
        match self {
            RPCMethod::BotApprove { .. } => RPCPerms::Staff,
            RPCMethod::BotDeny { .. } => RPCPerms::Staff,
            RPCMethod::BotVoteReset { .. } => RPCPerms::Owner,
            RPCMethod::BotVoteResetAll { .. } => RPCPerms::Owner,
            RPCMethod::BotUnverify { .. } => RPCPerms::Staff,
            RPCMethod::BotPremiumAdd { .. } => RPCPerms::Head,
            RPCMethod::BotPremiumRemove { .. } => RPCPerms::Head,
            RPCMethod::BotVoteBanAdd { .. } => RPCPerms::Head,
            RPCMethod::BotVoteBanRemove { .. } => RPCPerms::Head,
            RPCMethod::BotForceRemove { .. } => RPCPerms::Admin,
            RPCMethod::BotCertifyAdd { .. } => RPCPerms::Owner,
            RPCMethod::BotCertifyRemove { .. } => RPCPerms::Owner,
            RPCMethod::BotVoteCountSet { .. } => RPCPerms::Owner,
            RPCMethod::BotTransferOwnership { .. } => RPCPerms::Admin,
            RPCMethod::BotTransferOwnershipTeam { .. } => RPCPerms::Head,
        }
    }

    pub async fn handle(&self, state: RPCHandle) -> Result<RPCSuccess, Error> {
        // First ensure we have the permissions needed
        match self.needs_perms() {
            RPCPerms::Owner => {
                let staff_id_snow = state.user_id.parse::<NonZeroU64>()?;

                if !crate::config::CONFIG.owners.contains(&staff_id_snow) {
                    return Err("You need to be an owner to use this method".into());
                }
            }
            RPCPerms::Head => {
                let check = sqlx::query!(
                    "SELECT iblhdev, hadmin FROM users WHERE user_id = $1",
                    &state.user_id
                )
                .fetch_one(&state.pool)
                .await?;

                if !check.iblhdev && !check.hadmin {
                    return Err("You need to be at least a `Head Staff Manager` or a `Head Developer` to use this method".into());
                }
            }
            RPCPerms::Admin => {
                let check =
                    sqlx::query!("SELECT admin FROM users WHERE user_id = $1", &state.user_id)
                        .fetch_one(&state.pool)
                        .await?;

                if !check.admin {
                    return Err(
                        "You need to be at least a `Staff Manager` to use this method".into(),
                    );
                }
            }
            RPCPerms::Staff => {
                let check =
                    sqlx::query!("SELECT staff FROM users WHERE user_id = $1", &state.user_id)
                        .fetch_one(&state.pool)
                        .await?;

                if !check.staff {
                    return Err("You need to be a staff member to use this method".into());
                }
            }
        }

        // Also ensure that onboarding has happened
        let onboard_state = sqlx::query!(
            "SELECT staff, staff_onboard_state FROM users WHERE user_id = $1",
            &state.user_id
        )
        .fetch_one(&state.pool)
        .await?;

        if onboard_state.staff_onboard_state != "completed" {
            return Err("You need to complete onboarding in order to use RPC!".into());
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
                "UPDATE users SET api_token = $2 WHERE user_id = $1",
                &state.user_id,
                impls::crypto::gen_random(136)
            )
            .execute(&state.pool)
            .await
            .map_err(|_| "Failed to reset user token")?;

            return Err(
                "Rate limit exceeded. Wait 5-10 minutes, You will need to login/logout as well."
                    .into(),
            );
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
            RPCMethod::BotApprove { bot_id, reason } => {
                let claimed = sqlx::query!(
                    "SELECT type, claimed_by, last_claimed FROM bots WHERE bot_id = $1",
                    bot_id
                )
                .fetch_one(&state.pool)
                .await?;

                if claimed.r#type != "pending" {
                    return Err("Bot is not pending review?".into());
                }

                if claimed.claimed_by.is_none()
                    || claimed.claimed_by.as_ref().unwrap().is_empty()
                    || claimed.last_claimed.is_none()
                {
                    return Err(format!(
                        "<@{}> is not claimed? Do ``/claim`` to claim this bot first!",
                        bot_id
                    )
                    .into());
                }

                let start_time = chrono::offset::Utc::now();
                let last_claimed = claimed.last_claimed.unwrap();

                if (start_time - last_claimed).num_minutes() < 5 {
                    return Err("Whoa there! You need to test this bot for at least 5 minutes (recommended: 10-20 minutes) before being able to approve/deny it!".into());
                }

                // Find bot in testing server
                {
                    let guild = state
                        .cache_http
                        .cache
                        .guild(GuildId(crate::config::CONFIG.servers.testing))
                        .ok_or("Failed to find guild")?;

                    let member = guild.members.contains_key(&UserId(bot_id.parse()?));

                    if !member {
                        return Err("Bot is not in testing server. Please ensure this bot is in the testing server when approving. It will then be kicked by Arcadia when added to main server".into());
                    }
                }

                let ping = crate::impls::utils::resolve_ping_user(bot_id, &state.pool).await?;

                sqlx::query!(
                    "UPDATE bots SET type = 'approved', claimed_by = NULL WHERE bot_id = $1",
                    bot_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::default()
                    .content(format!("<@!{}>", ping))
                    .embed(
                        CreateEmbed::default()
                            .title("Bot Approved!")
                            .url(format!(
                                "{}/bots/{}",
                                crate::config::CONFIG.frontend_url,
                                bot_id
                            ))
                            .description(format!(
                                "<@!{}> has approved <@!{}>",
                                &state.user_id, bot_id
                            ))
                            .field("Feedback", reason, true)
                            .field("Moderator", "<@!".to_string() + &state.user_id + ">", true)
                            .field("Bot", "<@!".to_string() + bot_id + ">", true)
                            .footer(CreateEmbedFooter::new("Well done, young traveller!"))
                            .color(0x00ff00),
                    );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                let bot_owners = crate::impls::utils::get_bot_members(bot_id, &state.pool).await?;

                for owner in bot_owners {
                    let owner_snow = UserId(owner.parse()?);

                    let guild_id = GuildId(crate::config::CONFIG.servers.main);

                    if state
                        .cache_http
                        .cache
                        .member_field(guild_id, owner_snow, |m| m.user.id)
                        .is_some()
                    {
                        // Add role to user
                        if let Err(e) = state
                            .cache_http
                            .http
                            .add_member_role(
                                GuildId(crate::config::CONFIG.servers.main),
                                owner_snow,
                                RoleId(crate::config::CONFIG.roles.bot_developer),
                                Some("Autorole due to bots owned"),
                            )
                            .await
                        {
                            error!("Failed to add role to user: {}", e);
                        }
                    }
                }

                let invite_data = sqlx::query!("SELECT invite FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                Ok(RPCSuccess::Content(invite_data.invite))
            }
            RPCMethod::BotDeny { bot_id, reason } => {
                let claimed = sqlx::query!(
                    "SELECT type, claimed_by, owner, last_claimed FROM bots WHERE bot_id = $1",
                    bot_id
                )
                .fetch_one(&state.pool)
                .await?;

                if claimed.r#type != "pending" {
                    return Err("Bot is not pending review?".into());
                }

                if claimed.claimed_by.is_none()
                    || claimed.claimed_by.as_ref().unwrap().is_empty()
                    || claimed.last_claimed.is_none()
                {
                    return Err(format!(
                        "<@{}> is not claimed? Do ``/claim`` to claim this bot first!",
                        bot_id
                    )
                    .into());
                }

                let start_time = chrono::offset::Utc::now();
                let last_claimed = claimed.last_claimed.unwrap();

                if (start_time - last_claimed).num_minutes() < 5 {
                    return Err("Whoa there! You need to test this bot for at least 5 minutes (recommended: 10-20 minutes) before being able to approve/deny it!".into());
                }

                let ping = crate::impls::utils::resolve_ping_user(bot_id, &state.pool).await?;

                sqlx::query!(
                    "UPDATE bots SET type = 'denied', claimed_by = NULL WHERE bot_id = $1",
                    bot_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().content(format!("<@!{}>", ping)).embed(
                    CreateEmbed::default()
                        .title("Bot Denied!")
                        .url(format!(
                            "{}/bots/{}",
                            crate::config::CONFIG.frontend_url,
                            bot_id
                        ))
                        .description(format!("<@{}> has denied <@{}>", &state.user_id, bot_id))
                        .field("Reason", reason, true)
                        .field("Moderator", "<@!".to_string() + &state.user_id + ">", true)
                        .field("Bot", "<@!".to_string() + bot_id + ">", true)
                        .footer(CreateEmbedFooter::new(
                            "Well done, young traveller at getting denied from the club!",
                        ))
                        .color(0x00ff00),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteReset { bot_id, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                sqlx::query!("UPDATE bots SET votes = 0 WHERE bot_id = $1", bot_id)
                    .execute(&state.pool)
                    .await?;

                sqlx::query!("DELETE FROM votes WHERE bot_id = $1", bot_id)
                    .execute(&state.pool)
                    .await?;

                let msg = CreateMessage::default().embed(
                    CreateEmbed::default()
                        .title("__Bot Vote Reset!__")
                        .field("Reason", reason, true)
                        .field("Moderator", "<@".to_string() + &state.user_id + ">", true)
                        .field("Bot", "<@!".to_string() + bot_id + ">", true)
                        .footer(CreateEmbedFooter::new("Sad life :("))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteResetAll { reason } => {
                sqlx::query!("UPDATE bots SET votes = 0")
                    .execute(&state.pool)
                    .await?;

                sqlx::query!("DELETE FROM votes")
                    .execute(&state.pool)
                    .await?;

                let msg = CreateMessage::default().embed(
                    CreateEmbed::default()
                        .title("__All Votes Reset!__")
                        .field("Reason", reason, true)
                        .field("Moderator", "<@".to_string() + &state.user_id + ">", true)
                        .footer(CreateEmbedFooter::new("Sad life :("))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotUnverify { bot_id, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET type = 'pending', claimed_by = NULL WHERE bot_id = $1",
                    bot_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::default().embed(
                    CreateEmbed::default()
                        .title("__Bot Unverified For Futher Review!__")
                        .field("Reason", reason, true)
                        .field("Moderator", "<@".to_string() + &state.user_id + ">", true)
                        .field("Bot", "<@!".to_string() + bot_id + ">", true)
                        .footer(CreateEmbedFooter::new("Gonna be pending further review..."))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotPremiumAdd {
                bot_id,
                reason,
                time_period_hours,
            } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                // Set premium_period_length which is a postgres interval
                sqlx::query!(
                    "UPDATE bots SET start_premium_period = NOW(), premium_period_length = make_interval(hours => $1), premium = true WHERE bot_id = $2",
                    time_period_hours,
                    bot_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Premium Added!")
                        .description(format!(
                            "<@{}> has added premium to <@{}> for {} hours",
                            &state.user_id, bot_id, time_period_hours
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Well done, young traveller! Use it wisely...",
                        ))
                        .color(0x00ff00),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotPremiumRemove { bot_id, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                // Set premium_period_length which is a postgres interval
                sqlx::query!("UPDATE bots SET premium = false WHERE bot_id = $1", bot_id)
                    .execute(&state.pool)
                    .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Premium Removed!")
                        .description(format!(
                            "<@{}> has removed premium from <@{}>",
                            state.user_id, bot_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Well done, young traveller. Sad to see you go...",
                        ))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteBanAdd { bot_id, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET vote_banned = true WHERE bot_id = $1",
                    bot_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Vote Ban Edit!")
                        .description(format!(
                            "<@{}> has set the vote ban on <@{}>",
                            state.user_id, bot_id,
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Remember: don't abuse our services!",
                        ))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteBanRemove { bot_id, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET vote_banned = false WHERE bot_id = $1",
                    bot_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Vote Ban Removed!")
                        .description(format!(
                            "<@{}> has removed the vote ban on <@{}>",
                            state.user_id, bot_id,
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Remember: don't abuse our services!",
                        ))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotForceRemove {
                bot_id,
                reason,
                kick,
            } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                let bot_id_snow = bot_id.parse::<NonZeroU64>()?;

                if crate::config::CONFIG.protected_bots.contains(&bot_id_snow) && *kick {
                    return Err("You can't force delete this bot with 'kick' enabled!".into());
                }

                sqlx::query!("DELETE FROM bots WHERE bot_id = $1", bot_id)
                    .execute(&state.pool)
                    .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Bot Force Deleted!")
                        .description(format!(
                            "<@{}> has force-removed <@{}> for violating our rules or Discord ToS",
                            state.user_id, bot_id,
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Remember: don't abuse our services!",
                        ))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                if *kick {
                    // Check that the bot is in the server
                    let bot = state.cache_http.cache.member_field(
                        GuildId(crate::config::CONFIG.servers.main),
                        UserId(bot_id_snow),
                        |m| m.user.name.clone(),
                    );

                    if bot.is_some() {
                        GuildId(crate::config::CONFIG.servers.main)
                            .member(&state.cache_http, UserId(bot_id.parse()?))
                            .await?
                            .kick_with_reason(
                                &state.cache_http,
                                &(state.user_id.to_string() + ":" + reason),
                            )
                            .await?;
                    }
                }

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotCertifyAdd { bot_id, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET type = 'certified' WHERE bot_id = $1",
                    bot_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Bot Force Certified!")
                        .description(format!(
                            "<@{}> has force-certified <@{}>",
                            state.user_id, bot_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new("Neat"))
                        .color(0xff0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotCertifyRemove { bot_id, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET type = 'approved' WHERE bot_id = $1",
                    bot_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Bot Uncertified!")
                        .description(format!(
                            "<@{}> has uncertified <@{}>",
                            state.user_id, bot_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Uh oh, looks like you've been naughty...",
                        ))
                        .color(0xff0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteCountSet {
                bot_id,
                count,
                reason,
            } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                sqlx::query!(
                    "UPDATE bots SET votes = $2 WHERE bot_id = $1",
                    bot_id,
                    count
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Vote Count Updated!")
                        .description(format!(
                            "<@{}> has force-updated the vote count of <@{}>",
                            state.user_id, bot_id,
                        ))
                        .field("Reason", reason, true)
                        .field("New Vote Count", count.to_string(), true)
                        .footer(CreateEmbedFooter::new(
                            "Remember: don't abuse our services!",
                        ))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            },
            RPCMethod::BotTransferOwnership { bot_id, new_owner, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                // Check that the bot is not in a team
                let team_owner = sqlx::query!("SELECT team_owner FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if team_owner.team_owner.is_some() {
                    return Err("Bot is in a team. Please use BotTransferOwnershipTeam".into());
                }

                sqlx::query!(
                    "UPDATE bots SET owner = $2 WHERE bot_id = $1",
                    bot_id,
                    new_owner
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Bot Ownership Force Update!")
                        .description(format!(
                            "<@{}> has force-updated the ownership of <@{}> to <@{}>",
                            state.user_id, bot_id, new_owner
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Contact support if you think this is a mistake",
                        ))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            },
            RPCMethod::BotTransferOwnershipTeam { bot_id, new_team, reason } => {
                // Ensure the bot actually exists
                let bot = sqlx::query!("SELECT COUNT(*) FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if bot.count.unwrap_or_default() == 0 {
                    return Err("Bot does not exist".into());
                }

                // Parse the team ID
                let team_id = match new_team.parse::<Uuid>() {
                    Ok(id) => id,
                    Err(_) => return Err("Invalid team ID".into()),
                };

                // Check that the bot is not in a team
                let team_owner = sqlx::query!("SELECT team_owner FROM bots WHERE bot_id = $1", bot_id)
                    .fetch_one(&state.pool)
                    .await?;

                if team_owner.team_owner.is_none() {
                    return Err("Bot is not in a team. Please use BotTransferOwnership".into());
                }

                sqlx::query!(
                    "UPDATE bots SET team_owner = $2 WHERE bot_id = $1",
                    bot_id,
                    team_id
                )
                .execute(&state.pool)
                .await?;

                let msg = CreateMessage::new().embed(
                    CreateEmbed::default()
                        .title("Bot Ownership Force Update!")
                        .description(format!(
                            "<@{}> has force-updated the ownership of <@{}> to team {}",
                            state.user_id, bot_id, team_id
                        ))
                        .field("Reason", reason, true)
                        .footer(CreateEmbedFooter::new(
                            "Contact support if you think this is a mistake",
                        ))
                        .color(0xFF0000),
                );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(&state.cache_http, msg)
                    .await?;

                Ok(RPCSuccess::NoContent)
            }
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

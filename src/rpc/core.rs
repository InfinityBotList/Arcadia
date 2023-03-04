use sqlx::PgPool;
use strum_macros::{EnumVariantNames, EnumString, Display};
use ts_rs::TS;
use serde::Deserialize;

use crate::{impls, Error};

#[derive(Deserialize, TS)]
#[ts(export, export_to = ".generated/RPCRequest.ts")]
pub struct RPCRequest {
    pub user_id: String,
    pub token: String,
    pub method: RPCMethod,
    pub protocol: u8,
}

#[derive(Deserialize, TS, EnumString, EnumVariantNames, Display)]
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
    BotCertifyRemove {
        bot_id: String,
        reason: String,
    },
    BotVoteCountSet {
        bot_id: String,
        count: i32,
        reason: String,
    }
}

pub struct RPCHandle {
    pub pool: PgPool,
    pub cache_http: impls::cache::CacheHttpImpl,
    pub user_id: String,
}

impl RPCMethod {
    pub async fn handle(&self, state: RPCHandle) -> Result<RPCSuccess, Error> {
        match self {
            RPCMethod::BotApprove { bot_id, reason } => {
                let res = impls::actions::approve_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                )
                .await?;
    
                Ok(RPCSuccess::Content(res))
            }
            RPCMethod::BotDeny { bot_id, reason } => {
                impls::actions::deny_bot(&state.cache_http, &state.pool, bot_id, &state.user_id, reason).await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteReset { bot_id, reason } => {
                impls::actions::vote_reset_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteResetAll { reason } => {
                impls::actions::vote_reset_all_bot(
                    &state.cache_http,
                    &state.pool,
                    &state.user_id,
                    reason,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotUnverify { bot_id, reason } => {
                impls::actions::unverify_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotPremiumAdd {
                bot_id,
                reason,
                time_period_hours,
            } => {
                impls::actions::premium_add_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                    *time_period_hours,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotPremiumRemove { bot_id, reason } => {
                impls::actions::premium_remove_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteBanAdd { bot_id, reason } => {
                impls::actions::vote_ban_add_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotVoteBanRemove { bot_id, reason } => {
                impls::actions::vote_ban_remove_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotForceRemove {
                bot_id,
                reason,
                kick,
            } => {
                impls::actions::force_bot_remove(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                    *kick,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            }
            RPCMethod::BotCertifyRemove { bot_id, reason } => {
                impls::actions::certify_remove_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                )
                .await?;
    
                Ok(RPCSuccess::NoContent)
            },
            RPCMethod::BotVoteCountSet { bot_id, count, reason } => {
                impls::actions::vote_count_set_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &state.user_id,
                    reason,
                    *count,
                )
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

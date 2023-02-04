use libavacado::types::CacheHttpImpl;
use serde::{Deserialize};
use moka::future::Cache;
use sqlx::PgPool;

// For frontend API interface generation
use ts_rs::TS; 

pub struct AppState {
    pub cache_http: CacheHttpImpl,
    pub pool: PgPool,
    pub ratelimits: Cache<String, u64>,
}

#[derive(Deserialize, TS)]
#[ts(export, export_to="../.generated/RPCRequest.ts")]
pub struct RPCRequest {
    pub user_id: String,
    pub token: String,
    pub method: RPCMethod,
    pub protocol: u8,
}

#[derive(Deserialize, TS)]
#[ts(export, export_to="../.generated/RPCMethod.ts")]
pub enum RPCMethod {
    BotApprove { bot_id: String, reason: String },
    BotDeny { bot_id: String, reason: String },
    BotVoteReset { bot_id: String, reason: String },
    BotVoteResetAll { reason: String },
    BotUnverify { bot_id: String, reason: String },
}

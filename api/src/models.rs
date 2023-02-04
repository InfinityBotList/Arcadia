use std::sync::Arc;

use libavacado::public::AvacadoPublic;
use serde::{Deserialize};
use serenity::CacheAndHttp;
use moka::future::Cache;

pub struct AppState {
    pub pool: sqlx::PgPool,
    pub cache_http: Arc<CacheAndHttp>,
    pub avacado_public: Arc<AvacadoPublic>,
    pub ratelimits: Cache<String, u64>,
}

#[derive(Deserialize)]
pub struct RPCRequest {
    pub user_id: String,
    pub token: String,
    pub method: RPCMethod,
    pub protocol: u8,
}

#[derive(Deserialize)]
pub enum RPCMethod {
    BotApprove { bot_id: String, reason: String },
    BotDeny { bot_id: String, reason: String },
    BotVoteReset { bot_id: String, reason: String },
    BotVoteResetAll { reason: String },
    BotUnverify { bot_id: String, reason: String },
}

use std::sync::Arc;

use libavacado::public::AvacadoPublic;
use serde::{Deserialize, Serialize};
use serenity::CacheAndHttp;

pub struct AppState {
    pub pool: sqlx::PgPool,
    pub cache_http: Arc<CacheAndHttp>,
    pub avacado_public: Arc<AvacadoPublic>,
}

#[derive(Serialize, Deserialize)]
pub struct APIResponse {
    pub done: bool,
    pub reason: String,
    pub context: Option<String>,
}

#[derive(Deserialize)]
pub struct OauthReq {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
pub struct OauthRes {
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct OauthUser {
    pub id: String,
    pub username: String,
    pub avatar: Option<String>,
    pub discriminator: String,
}
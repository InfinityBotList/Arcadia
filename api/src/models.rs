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

#[derive(Deserialize)]
pub struct SVQuery {
    pub uid: String,
    pub frag: String,
}

#[derive(Deserialize)]
pub struct SVODQuery {
    pub code: String,
}

#[derive(Deserialize)]
pub struct Request {
    pub staff_id: String,
    pub bot_id: String,
    pub reason: String,
}

#[derive(Deserialize)]
pub struct GenericRequest {
    pub staff_id: String,
    pub reason: String,
}

#[derive(Deserialize)]
pub struct UserRequest {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct CreateAppQuery {
    pub user_id: String,
    pub position: String,
}

#[derive(Deserialize)]
pub struct GetAppQuery {
    pub app_id: String,
    pub user_id: String,
}
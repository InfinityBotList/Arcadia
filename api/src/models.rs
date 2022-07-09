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

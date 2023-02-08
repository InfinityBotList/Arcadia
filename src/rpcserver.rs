use std::{net::SocketAddr, time::Duration, ops::Add};

use axum::{
    routing::{post},
    http::{StatusCode, self},
    response::{IntoResponse, Response},
    Json, Router,
    extract::State
};
use log::info;
use reqwest::Method;
use serde::{Deserialize};
use moka::future::Cache;
use sqlx::PgPool;
use crate::impls::cache::CacheHttpImpl;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use crate::{impls, config};

// For frontend API interface generation
use ts_rs::TS;

#[derive(Deserialize, TS)]
#[ts(export, export_to=".generated/RPCRequest.ts")]
pub struct RPCRequest {
    pub user_id: String,
    pub token: String,
    pub method: RPCMethod,
    pub protocol: u8,
}

#[derive(Deserialize, TS)]
#[ts(export, export_to=".generated/RPCMethod.ts")]
pub enum RPCMethod {
    BotApprove { bot_id: String, reason: String }, // Added
    BotDeny { bot_id: String, reason: String }, // Added
    BotVoteReset { bot_id: String, reason: String }, // Added
    BotVoteResetAll { reason: String },
    BotUnverify { bot_id: String, reason: String }, // Added
}

pub enum RPCResponse {
    NoContent,
    Content(String),
    Err(String),
    InvalidProtocol,
    Ratelimited,
    UserNotFound,
    InvalidAuth,
    StaffOnly,
    PermissionDenied(Vec<&'static str>)
}

impl IntoResponse for RPCResponse {
    fn into_response(self) -> Response {
        match self {
            Self::NoContent => (StatusCode::NO_CONTENT, "").into_response(),
            Self::Content(content) => (StatusCode::OK, content).into_response(),
            Self::Err(err) => (StatusCode::BAD_REQUEST, err).into_response(),
            Self::InvalidProtocol => (StatusCode::PRECONDITION_FAILED, "Invalid protocol").into_response(),
            Self::Ratelimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded. Wait 5-10 minutes, You will need to login/logout as well.").into_response(),
            Self::UserNotFound => (StatusCode::NOT_FOUND, "This user could not be found").into_response(),
            Self::InvalidAuth => (StatusCode::UNAUTHORIZED, "Invalid auth. Logout and login again to get a new token.").into_response(),
            Self::StaffOnly => (StatusCode::FORBIDDEN, "Staff-only endpoint").into_response(),
            Self::PermissionDenied(perms) => (StatusCode::FORBIDDEN, "Permission denied: ".to_string() + &perms.join(" ").to_string()).into_response(),
        }
    }
}

pub struct AppState {
    pub cache_http: CacheHttpImpl,
    pub pool: PgPool,
    pub ratelimits: Cache<String, u64>,
}

pub async fn rpc_init(
    pool: PgPool,
    cache_http: CacheHttpImpl,
) {
    let shared_state = Arc::new(AppState {
        pool,
        cache_http,
        ratelimits: moka::future::Cache::builder()
        // Time to live (TTL): 7 minutes
        .time_to_live(Duration::from_secs(60 * 7))
        // Create the cache.
        .build(),        
    });

    let mut origins = vec![];

    for origin in config::CONFIG.rpc_allowed_urls.iter() {
        origins.push(origin.parse().unwrap());
    }

    let app = Router::new()
    .route("/", post(web_rpc_api))
    .with_state(shared_state)
    .layer(
        CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET])
        .allow_headers([http::header::CONTENT_TYPE])
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3010));

    info!("Starting RPC server on {}", addr);

    axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();
}

async fn web_rpc_api(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RPCRequest>,
) -> impl IntoResponse {
    if req.protocol != 2 {
        return RPCResponse::InvalidProtocol;
    }

    let check = sqlx::query!(
        "SELECT staff, ibldev, iblhdev, admin, hadmin, api_token FROM users WHERE user_id = $1",
        &req.user_id
    )
    .fetch_one(&state.pool)
    .await;

    if check.is_err() {
        return RPCResponse::UserNotFound;
    }

    let check = check.unwrap();

    if check.api_token != req.token {
        return RPCResponse::InvalidAuth;
    }

    if !check.staff {
        return RPCResponse::StaffOnly;
    }

    // Add request to moka cache
    let new_req = state.ratelimits.get(&req.user_id).unwrap_or_default().add(1);

    state.ratelimits.insert(req.user_id.clone(), new_req).await;

    if new_req > 6 {
        let res =  sqlx::query!(
            "UPDATE users SET api_token = $2 WHERE user_id = $1",
            &req.user_id,
            impls::crypto::gen_random(136)
        )
        .execute(&state.pool)
        .await;

        if res.is_err() {
            return RPCResponse::Err("Failed to reset user token (caused by ratelimit)".to_string());
        }

        return RPCResponse::Ratelimited;
    }

    match &req.method {
        RPCMethod::BotApprove { bot_id, reason } => {
            let res = impls::actions::approve_bot(
                &state.cache_http,
                &state.pool,
                &bot_id,
                &req.user_id,
                &reason,
            )
            .await;

            if res.is_err() {
                RPCResponse::Err(res.unwrap_err().to_string())
            } else {
                RPCResponse::Content(res.unwrap())
            }
        }
        RPCMethod::BotDeny { bot_id, reason } => {
            let err = impls::actions::deny_bot(
                &state.cache_http,
                &state.pool,
                &bot_id,
                &req.user_id,
                &reason,
            )
            .await;

            if err.is_err() {
                RPCResponse::Err(err.unwrap_err().to_string())
            } else {
                RPCResponse::NoContent
            }
        }
        RPCMethod::BotVoteReset { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"])
            } else {
                let err = impls::actions::vote_reset_bot(
                    &state.cache_http,
                    &state.pool,
                    &bot_id,
                    &req.user_id,
                    &reason,
                )
                .await;

                if err.is_err() {
                    RPCResponse::Err(err.unwrap_err().to_string())
                } else {
                    RPCResponse::NoContent
                }
            }
        }
        RPCMethod::BotVoteResetAll { reason } => {
            if !(check.hadmin || check.iblhdev) {
                RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"])
            } else {
                let err = impls::actions::vote_reset_all_bot(
                    &state.cache_http,
                    &state.pool,
                    &req.user_id,
                    &reason,
                )
                .await;

                if err.is_err() {
                    RPCResponse::Err(err.unwrap_err().to_string())
                } else {
                    RPCResponse::NoContent
                }
            }
        },
        RPCMethod::BotUnverify { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"])
            } else {
                let err = impls::actions::unverify_bot(
                    &state.cache_http,
                    &state.pool,
                    &bot_id,
                    &req.user_id,
                    &reason,
                )
                .await;
            
                if err.is_err() {
                    RPCResponse::Err(err.unwrap_err().to_string())
                } else {
                    RPCResponse::NoContent
                }    
            }
        },
    }
}
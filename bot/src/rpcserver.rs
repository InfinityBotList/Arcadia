use std::{net::SocketAddr, time::Duration, ops::Add};

use axum::{
    routing::{post},
    http::{StatusCode, self},
    response::IntoResponse,
    Json, Router,
    extract::State
};
use log::info;
use reqwest::Method;
use serde::{Deserialize};
use moka::future::Cache;
use sqlx::PgPool;
use libavacado::types::CacheHttpImpl;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

// For frontend API interface generation
use ts_rs::TS;

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
    BotApprove { bot_id: String, reason: String }, // Added
    BotDeny { bot_id: String, reason: String }, // Added
    BotVoteReset { bot_id: String, reason: String }, // Added
    BotVoteResetAll { reason: String },
    BotUnverify { bot_id: String, reason: String }, // Added
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

    for origin in libavacado::CONFIG.rpc_allowed_urls.iter() {
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
        return (StatusCode::BAD_REQUEST, "Invalid protocol version".to_string());
    }

    let check = sqlx::query!(
        "SELECT staff, ibldev, iblhdev, admin, hadmin, api_token FROM users WHERE user_id = $1",
        &req.user_id
    )
    .fetch_one(&state.pool)
    .await;

    if check.is_err() {
        return (StatusCode::UNAUTHORIZED, "User not found".to_string());
    }

    let check = check.unwrap();

    if check.api_token != req.token {
        return (StatusCode::UNAUTHORIZED, "Invalid token. Logout and login again to get a new token.".to_string());
    }

    if !check.staff {
        return (StatusCode::UNAUTHORIZED, "Staff-only endpoint".to_string());
    }

    // Add request to moka cache
    let new_req = state.ratelimits.get(&req.user_id).unwrap_or_default().add(1);

    state.ratelimits.insert(req.user_id.clone(), new_req).await;

    if new_req > 6 {
        let res =  sqlx::query!(
            "UPDATE users SET api_token = $2 WHERE user_id = $1",
            &req.user_id,
            libavacado::crypto::gen_random(136)
        )
        .execute(&state.pool)
        .await;

        if res.is_err() {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to reset user token (caused by ratelimit)".to_string());
        }

        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded. Wait 5-10 minutes, You will need to login/logout as well.".to_string());
    }

    match &req.method {
        RPCMethod::BotApprove { bot_id, reason } => {
            let res = libavacado::staff::approve_bot(
                &state.cache_http,
                &state.pool,
                &bot_id,
                &req.user_id,
                &reason,
            )
            .await;

            if res.is_err() {
                (StatusCode::BAD_REQUEST, res.unwrap_err().to_string())
            } else {
                (StatusCode::OK, res.unwrap().invite)
            }
        }
        RPCMethod::BotDeny { bot_id, reason } => {
            let err = libavacado::staff::deny_bot(
                &state.cache_http,
                &state.pool,
                &bot_id,
                &req.user_id,
                &reason,
            )
            .await;

            if err.is_err() {
                (StatusCode::BAD_REQUEST, err.unwrap_err().to_string())
            } else {
                (StatusCode::NO_CONTENT, "".to_string())
            }
        }
        RPCMethod::BotVoteReset { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                (StatusCode::UNAUTHORIZED, "Permission denied".to_string())
            } else {
                let err = libavacado::manage::vote_reset_bot(
                    &state.cache_http,
                    &state.pool,
                    &bot_id,
                    &req.user_id,
                    &reason,
                )
                .await;

                if err.is_err() {
                    (StatusCode::BAD_REQUEST, err.unwrap_err().to_string())
                } else {
                    (StatusCode::NO_CONTENT, "".to_string())
                }
            }
        }
        RPCMethod::BotVoteResetAll { reason } => {
            if !(check.hadmin || check.iblhdev) {
                (StatusCode::UNAUTHORIZED, "Permission denied".to_string())
            } else {
                let err = libavacado::manage::vote_reset_all_bot(
                    &state.cache_http,
                    &state.pool,
                    &req.user_id,
                    &reason,
                )
                .await;

                if err.is_err() {
                    (StatusCode::BAD_REQUEST, err.unwrap_err().to_string())
                } else {
                    (StatusCode::NO_CONTENT, "".to_string())
                }    
            }
        },
        RPCMethod::BotUnverify { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                (StatusCode::UNAUTHORIZED, "Permission denied".to_string())
            } else {
                let err = libavacado::manage::unverify_bot(
                    &state.cache_http,
                    &state.pool,
                    &bot_id,
                    &req.user_id,
                    &reason,
                )
                .await;
            
                if err.is_err() {
                    (StatusCode::BAD_REQUEST, err.unwrap_err().to_string())
                } else {
                    (StatusCode::NO_CONTENT, "".to_string())
                }    
            }
        },
    }
}
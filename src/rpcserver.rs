use std::num::NonZeroU64;

use crate::impls::cache::CacheHttpImpl;
use crate::{config, impls};
use axum::{
    extract::State,
    http::{self, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::Utc;
use log::info;
use reqwest::Method;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

// For frontend API interface generation
use ts_rs::TS;

#[derive(Deserialize, TS)]
#[ts(export, export_to = ".generated/RPCRequest.ts")]
pub struct RPCRequest {
    pub user_id: String,
    pub token: String,
    pub method: RPCMethod,
    pub protocol: u8,
}

#[derive(Deserialize, TS)]
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
    }
}

impl ToString for RPCMethod {
    fn to_string(&self) -> String {
        match self {
            Self::BotApprove { .. } => "BotApprove",
            Self::BotDeny { .. } => "BotDeny",
            Self::BotVoteReset { .. } => "BotVoteReset",
            Self::BotVoteResetAll { .. } => "BotVoteResetAll",
            Self::BotUnverify { .. } => "BotUnverify",
            Self::BotPremiumAdd { .. } => "BotPremiumAdd",
            Self::BotPremiumRemove { .. } => "BotPremiumRemove",
            Self::BotVoteBanAdd { .. } => "BotVoteBanAdd",
            Self::BotVoteBanRemove { .. } => "BotVoteBanRemove",
            Self::BotForceRemove { .. } => "BotForceRemove",
            Self::BotCertifyRemove { .. } => "BotCertifyRemove",
        }
        .to_string()
    }
}

/// Dead code as compilation check/requirement
#[allow(dead_code)]
impl RPCMethod {
    fn associated_cmd(&self) {
        match self {
            Self::BotApprove { .. } => crate::testing::approve,
            Self::BotDeny { .. } => crate::testing::deny,
            Self::BotVoteReset { .. } => crate::admin::botvotereset,
            Self::BotVoteResetAll { .. } => crate::admin::botvoteresetall,
            Self::BotUnverify { .. } => crate::admin::botunverify,
            Self::BotPremiumAdd { .. } => crate::admin::botpremiumadd,
            Self::BotPremiumRemove { .. } => crate::admin::botpremiumdel,
            Self::BotVoteBanAdd { .. } => crate::admin::botvotebanadd,
            Self::BotVoteBanRemove { .. } => crate::admin::botvotebandel,
            Self::BotForceRemove { .. } => crate::admin::botforcedel,
            Self::BotCertifyRemove { .. } => crate::admin::botuncertify,
        };
    }
}

pub enum RPCSuccess {
    NoContent,
    Content(String),
}

impl IntoResponse for RPCSuccess {
    fn into_response(self) -> Response {
        match self {
            Self::NoContent => (StatusCode::NO_CONTENT, "").into_response(),
            Self::Content(content) => (StatusCode::OK, content).into_response(),
        }
    }
}

pub enum RPCResponse {
    Err(String),
    InvalidProtocol,
    RPCLocked,
    RatelimitReqFindFail,
    RatelimitAddFail,
    RatelimitUserTokenResetFail,
    Ratelimited,
    UserNotFound,
    InvalidAuth,
    StaffOnly,
    PermissionDenied(Vec<&'static str>),
}

impl IntoResponse for RPCResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Err(err) => (StatusCode::BAD_REQUEST, err).into_response(),
            Self::InvalidProtocol => {
                (StatusCode::PRECONDITION_FAILED, "Invalid protocol").into_response()
            }
            Self::RPCLocked => (
                StatusCode::PRECONDITION_FAILED,
                "RPC is locked. Use `rpcunlock` to unlock it for 1 hour",
            )
                .into_response(),
            Self::RatelimitAddFail => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to add request to rpc_requests table",
            )
                .into_response(),
            Self::RatelimitReqFindFail => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get number of requests in the last 7 minutes",
            )
                .into_response(),
            Self::RatelimitUserTokenResetFail => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to reset user token (caused by ratelimit)",
            )
                .into_response(),
            Self::Ratelimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded. Wait 5-10 minutes, You will need to login/logout as well.",
            )
                .into_response(),
            Self::UserNotFound => {
                (StatusCode::NOT_FOUND, "This user could not be found").into_response()
            }
            Self::InvalidAuth => (
                StatusCode::UNAUTHORIZED,
                "Invalid auth. Logout and login again to get a new token.",
            )
                .into_response(),
            Self::StaffOnly => (StatusCode::FORBIDDEN, "Staff-only endpoint").into_response(),
            Self::PermissionDenied(perms) => (
                StatusCode::FORBIDDEN,
                "Permission denied: ".to_string() + &perms.join(" "),
            )
                .into_response(),
        }
    }
}

pub struct AppState {
    pub cache_http: CacheHttpImpl,
    pub pool: PgPool,
}

pub async fn rpc_init(pool: PgPool, cache_http: CacheHttpImpl) {
    let shared_state = Arc::new(AppState { pool, cache_http });

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
                .allow_headers([http::header::CONTENT_TYPE]),
        );

    let addr = "127.0.0.1:3010"
        .parse()
        .expect("Invalid RPC server address");

    info!("Starting RPC server on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn web_rpc_api(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RPCRequest>,
) -> Result<RPCSuccess, RPCResponse> {
    if req.protocol != 3 {
        return Err(RPCResponse::InvalidProtocol);
    }

    let check = sqlx::query!(
        "SELECT staff, ibldev, iblhdev, admin, hadmin, api_token, staff_rpc_last_verify FROM users WHERE user_id = $1",
        &req.user_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| RPCResponse::UserNotFound)?;

    if check.api_token != req.token {
        return Err(RPCResponse::InvalidAuth);
    }

    if !check.staff {
        return Err(RPCResponse::StaffOnly);
    }

    match &req.method {
        RPCMethod::BotApprove { .. } => {}
        RPCMethod::BotDeny { .. } => {}
        _ => {
            if Utc::now().timestamp() - check.staff_rpc_last_verify.timestamp() > 600 {
                return Err(RPCResponse::RPCLocked);
            }
        }
    }

    let user_id_snowflake = req
        .user_id
        .parse::<NonZeroU64>()
        .map_err(|_| RPCResponse::UserNotFound)?;

    // Add request to rpc_requests table
    sqlx::query!(
        "INSERT INTO rpc_requests (user_id, method) VALUES ($1, $2)",
        &req.user_id,
        &req.method.to_string()
    )
    .execute(&state.pool)
    .await
    .map_err(|_| RPCResponse::RatelimitAddFail)?;

    // Get number of requests in the last 7 minutes
    let res = sqlx::query!(
        "SELECT COUNT(*) FROM rpc_requests WHERE user_id = $1 AND NOW() - created_at < INTERVAL '7 minutes'",
        &req.user_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| RPCResponse::RatelimitReqFindFail)?;

    let count = res.count.unwrap_or_default();

    if count > 5 {
        sqlx::query!(
            "UPDATE users SET api_token = $2 WHERE user_id = $1",
            &req.user_id,
            impls::crypto::gen_random(136)
        )
        .execute(&state.pool)
        .await
        .map_err(|_| RPCResponse::RatelimitUserTokenResetFail)?;

        return Err(RPCResponse::Ratelimited);
    }

    match &req.method {
        RPCMethod::BotApprove { bot_id, reason } => {
            let res = impls::actions::approve_bot(
                &state.cache_http,
                &state.pool,
                bot_id,
                &req.user_id,
                reason,
            )
            .await
            .map_err(|e| RPCResponse::Err(e.to_string()))?;

            Ok(RPCSuccess::Content(res))
        }
        RPCMethod::BotDeny { bot_id, reason } => {
            impls::actions::deny_bot(&state.cache_http, &state.pool, bot_id, &req.user_id, reason)
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

            Ok(RPCSuccess::NoContent)
        }
        RPCMethod::BotVoteReset { bot_id, reason } => {
            if !config::CONFIG.owners.contains(&user_id_snowflake) {
                Err(RPCResponse::PermissionDenied(vec!["owner"]))
            } else {
                impls::actions::vote_reset_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &req.user_id,
                    reason,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
        RPCMethod::BotVoteResetAll { reason } => {
            if !config::CONFIG.owners.contains(&user_id_snowflake) {
                Err(RPCResponse::PermissionDenied(vec!["owner"]))
            } else {
                impls::actions::vote_reset_all_bot(
                    &state.cache_http,
                    &state.pool,
                    &req.user_id,
                    reason,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
        RPCMethod::BotUnverify { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                Err(RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"]))
            } else {
                impls::actions::unverify_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &req.user_id,
                    reason,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
        RPCMethod::BotPremiumAdd {
            bot_id,
            reason,
            time_period_hours,
        } => {
            if !(check.hadmin || check.iblhdev) {
                Err(RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"]))
            } else {
                impls::actions::premium_add_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &req.user_id,
                    reason,
                    *time_period_hours,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
        RPCMethod::BotPremiumRemove { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                Err(RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"]))
            } else {
                impls::actions::premium_remove_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &req.user_id,
                    reason,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
        RPCMethod::BotVoteBanAdd { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                Err(RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"]))
            } else {
                impls::actions::vote_ban_add_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &req.user_id,
                    reason,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
        RPCMethod::BotVoteBanRemove { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                Err(RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"]))
            } else {
                impls::actions::vote_ban_remove_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &req.user_id,
                    reason,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
        RPCMethod::BotForceRemove {
            bot_id,
            reason,
            kick,
        } => {
            if !(check.hadmin || check.iblhdev) {
                Err(RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"]))
            } else {
                impls::actions::force_bot_remove(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &req.user_id,
                    reason,
                    *kick,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
        RPCMethod::BotCertifyRemove { bot_id, reason } => {
            if !(check.hadmin || check.iblhdev) {
                Err(RPCResponse::PermissionDenied(vec!["hadmin", "iblhdev"]))
            } else {
                impls::actions::certify_remove_bot(
                    &state.cache_http,
                    &state.pool,
                    bot_id,
                    &req.user_id,
                    reason,
                )
                .await
                .map_err(|e| RPCResponse::Err(e.to_string()))?;

                Ok(RPCSuccess::NoContent)
            }
        }
    }
}

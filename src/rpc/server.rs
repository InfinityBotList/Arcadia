use std::sync::Arc;

use crate::impls;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use log::info;
use reqwest::Method;
use sqlx::PgPool;
use tower_http::cors::{CorsLayer, Any};

use super::core::{RPCHandle, RPCMethod, RPCRequest, RPCSuccess};
use chrono::Utc;

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
}

pub enum Success {
    Content(String),
    NoContent,
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
        }
    }
}

impl IntoResponse for Success {
    fn into_response(self) -> Response {
        match self {
            Self::Content(content) => (StatusCode::OK, content).into_response(),
            Self::NoContent => (StatusCode::NO_CONTENT, "").into_response(),
        }
    }
}

pub struct AppState {
    pub cache_http: impls::cache::CacheHttpImpl,
    pub pool: PgPool,
}

pub async fn rpc_init(pool: PgPool, cache_http: impls::cache::CacheHttpImpl) {
    let shared_state = Arc::new(AppState { pool, cache_http });

    let app = Router::new()
        .route("/", post(web_rpc_api))
        .with_state(shared_state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = "127.0.0.1:3010"
        .parse()
        .expect("Invalid RPC server address");

    info!("Starting RPC server on {}", addr);

    if let Err(e) = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
    {
        panic!("RPC server error: {}", e);
    }
}

async fn web_rpc_api(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RPCRequest>,
) -> Result<Success, RPCResponse> {
    if req.protocol != 3 {
        return Err(RPCResponse::InvalidProtocol);
    }

    let check = sqlx::query!(
        "SELECT staff, api_token, staff_rpc_last_verify FROM users WHERE user_id = $1",
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

    match req
        .method
        .handle(RPCHandle {
            cache_http: state.cache_http.clone(),
            pool: state.pool.clone(),
            user_id: req.user_id,
        })
        .await
        .map_err(|e| RPCResponse::Err(e.to_string()))?
    {
        RPCSuccess::Content(content) => Ok(Success::Content(content)),
        RPCSuccess::NoContent => Ok(Success::NoContent),
    }
}

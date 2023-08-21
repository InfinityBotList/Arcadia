use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::impls;
use crate::rpc::core::{RPCField, FieldType};
use axum::http::HeaderMap;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use log::info;
use sqlx::PgPool;
use strum::VariantNames;
use tower_http::cors::{Any, CorsLayer};

use super::core::{RPCMethod, RPCPerms};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use once_cell::sync::Lazy;
use moka::future::Cache;

#[derive(Clone)]
pub struct KeychainData {
    pub user_id: String,
}

pub static RPC_WEB_CHAIN: Lazy<Cache<String, KeychainData>> = Lazy::new(|| {
    info!("RPC_WEB_CHAIN initialized");

    Cache::builder()
        // Time to live (TTL): 15 minutes
        .time_to_live(Duration::from_secs(5 * 60))        // Create the cache.
        .build()
});

#[derive(Deserialize, ToSchema, TS)]
#[ts(export, export_to = ".generated/RPCRequest.ts")]
pub struct RPCRequest {
    pub user_id: String,
    pub method: RPCMethod,
    pub api_token: String,
    pub rpc_identity: String,
    pub protocol: u8,
}

pub enum RPCResponse {
    Err(String),
    InvalidProtocol,
    InvalidIdentity,
    UsageQuoteExceeded,
    MethodNotAllowed,
    UserNotFound,
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
            Self::InvalidProtocol => (
                StatusCode::PRECONDITION_FAILED,
                "Out of date client. Please use the bot until this is fixed",
            )
                .into_response(),
            Self::UserNotFound => {
                (StatusCode::NOT_FOUND, "This user could not be found. Try logging out and logging in again?").into_response()
            }
            Self::InvalidIdentity => (
                StatusCode::UNAUTHORIZED,
                "Invalid RPC identity. Generate a new one?",
            )
                .into_response(),
            Self::UsageQuoteExceeded => (StatusCode::TOO_MANY_REQUESTS, "Usage quotas exceeded for this RPC identity, generate another one?").into_response(),
            Self::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed for this RPC identity").into_response(),
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
    use utoipa::OpenApi;
    #[derive(OpenApi)]
    #[openapi(paths(web_rpc_api, available_actions), components(schemas(RPCRequest, WebAction, RPCField, RPCMethod, RPCPerms, FieldType)))]
    struct ApiDoc;  

    async fn docs() -> impl IntoResponse {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        let data = ApiDoc::openapi().to_json();

        if let Ok(data) = data {
            return (headers, data).into_response();
        }

        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate docs".to_string()).into_response()
    }  
    
    let shared_state = Arc::new(AppState { pool, cache_http });

    let app = Router::new()
        .route("/openapi", get(docs))
        .route("/", post(web_rpc_api))
        .route("/actions", get(available_actions))
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

/// Create Staff RPC
///
/// This is the main API exposed by RPC. It is used to perform staff actions
#[utoipa::path(
    post,
    request_body = RPCRequest,
    path = "/",
    responses(
        (status = 200, description = "Content", body = String),
        (status = 204, description = "No content"),
        (status = PRECONDITION_FAILED, description = "Out of date client. Please use the bot until this is fixed", body = String),
        (status = TOO_MANY_REQUESTS, description = "Usage quotas exceeded for this RPC identity, generate another one?", body = String),
        (status = BAD_REQUEST, description = "An error occured", body = String),
        (status = NOT_FOUND, description = "Not Found Error", body = String)
    ),
)]
async fn web_rpc_api(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RPCRequest>,
) -> Result<Success, RPCResponse> {
    Err(RPCResponse::Err("RPC is currently disabled".to_string()))

    /*
    // Check RPC key
    let keychain = RPC_KEYCHAIN.get(&req.rpc_identity);

    if keychain.is_none() {
        return Err(RPCResponse::InvalidIdentity);
    }

    let keychain = keychain.unwrap();

    // Ensure it matches user
    if keychain.user_id != req.user_id {
        return Err(RPCResponse::InvalidIdentity);
    }

    // Check usage limits
    if keychain.used > keychain.max_uses {
        return Err(RPCResponse::UsageQuoteExceeded);
    }

    // Get name of method
    if !keychain.allowed_methods.contains(&req.method.to_string()) {
        return Err(RPCResponse::MethodNotAllowed);
    }

    // Increment used
    RPC_KEYCHAIN.insert(req.rpc_identity,  KeychainData {
        used: keychain.used + 1,
        ..keychain
    }).await;

    let check = sqlx::query!(
        "SELECT staff FROM users WHERE user_id = $1 AND api_token = $2",
        &req.user_id,
        &req.api_token
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| RPCResponse::UserNotFound)?;

    if !check.staff {
        return Err(RPCResponse::StaffOnly);
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
    }*/
}

#[derive(Serialize, ToSchema, TS)]
#[ts(export, export_to = ".generated/RPCWebAction.ts")]
struct WebAction {
    id: String,
    label: String,
    description: String,
    needed_perms: RPCPerms,
    fields: Vec<RPCField>,
}

#[derive(Deserialize)]
struct WebActionQuery {
    user_id: Option<String>,
}

/// Get Available Actions
/// 
/// This is used to render the list of fields to display for a given RPC method
#[utoipa::path(
    get,
    path = "/actions",
    responses(
        (status = 200, description = "RPC WebField Data", body = Vec<WebAction>),
        (status = BAD_REQUEST, description = "An error occured", body = String),
        (status = NOT_FOUND, description = "Not Found Error", body = String)
    ),
)]
async fn available_actions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WebActionQuery>,
) -> Result<Json<Vec<WebAction>>, RPCResponse> {
    let (owner, head, admin, staff) = if let Some(id) = query.user_id {
        let count = sqlx::query!("SELECT COUNT(*) FROM users WHERE user_id = $1", id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| RPCResponse::Err(e.to_string()))?;

        if count.count.unwrap_or_default() == 0 {
            return Err(RPCResponse::UserNotFound);
        }

        let perms = sqlx::query!(
            "SELECT owner, hadmin, iblhdev, admin, staff FROM users WHERE user_id = $1",
            id
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| RPCResponse::Err(e.to_string()))?;

        (
            perms.owner,
            perms.hadmin || perms.iblhdev,
            perms.admin,
            perms.staff,
        )
    } else {
        (true, true, true, true)
    };

    let mut actions = Vec::new();

    for variant in super::core::RPCMethod::VARIANTS {
        let method = super::core::RPCMethod::from_str(variant)
            .map_err(|e| RPCResponse::Err(e.to_string()))?;

        let action = WebAction {
            id: variant.to_string(),
            label: method.label(),
            description: method.description(),
            needed_perms: method.needs_perms(),
            fields: method.method_fields(),
        };

        match action.needed_perms {
            RPCPerms::Owner => {
                if owner {
                    actions.push(action);
                }
            }
            RPCPerms::Head => {
                if head {
                    actions.push(action);
                }
            }
            RPCPerms::Admin => {
                if admin {
                    actions.push(action);
                }
            }
            RPCPerms::Staff => {
                if staff {
                    actions.push(action);
                }
            }
        }
    }

    Ok(Json(actions))
}

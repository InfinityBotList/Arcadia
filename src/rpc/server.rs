use std::sync::Arc;
use std::str::FromStr;

use crate::impls;
use axum::{
    extract::{State, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use log::info;
use sqlx::PgPool;
use strum::VariantNames;
use tower_http::cors::{Any, CorsLayer};

use super::core::{RPCHandle, RPCMethod, RPCSuccess, RPCPerms};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ts_rs::TS;


#[derive(Deserialize, TS)]
#[ts(export, export_to = ".generated/RPCRequest.ts")]
pub struct RPCRequest {
    pub user_id: String,
    pub token: String,
    pub method: RPCMethod,
    pub protocol: u8,
}

pub enum RPCResponse {
    Err(String),
    InvalidProtocol,
    RPCLocked,
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
                (StatusCode::PRECONDITION_FAILED, "Out of date client. Please use the bot until this is fixed").into_response()
            },
            Self::RPCLocked => (
                StatusCode::PRECONDITION_FAILED,
                "RPC is locked. Use `rpcunlock` to unlock it for 1 hour",
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

async fn web_rpc_api(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RPCRequest>,
) -> Result<Success, RPCResponse> {
    if req.protocol != 4 {
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

#[derive(Serialize, TS)]
#[ts(export, export_to = ".generated/RPCWebField.ts")]
struct WebField {
    id: String, 
    label: String, 
    field_type: FieldType,
    icon: String,
    placeholder: String,
}

impl WebField {
    fn bot_id() -> Self {
        WebField {
            id: "bot_id".to_string(),
            label: "Bot ID".to_string(),
            field_type: FieldType::Text,
            icon: "ic:twotone-access-time-filled".to_string(),
            placeholder: "The Bot ID to perform the action on".to_string(),
        }
    }

    fn reason() -> Self {
        WebField {
            id: "reason".to_string(),
            label: "Reason".to_string(),
            field_type: FieldType::Textarea,
            icon: "material-symbols:question-mark".to_string(),
            placeholder: "Reason for performing this action".to_string(),
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export, export_to = ".generated/RPCFieldType.ts")]
enum FieldType {
    Text,
    Textarea,
    Number,
    Hour, // Time expressed as a number of hours
    Boolean 
}

// Returns a set of WebField's for a given enum variant
fn method_web_fields(method: RPCMethod) -> Vec<WebField> {
    match method {
        RPCMethod::BotApprove { .. } => vec![
            WebField::bot_id(),
            WebField::reason()
        ],
        RPCMethod::BotDeny { .. } => vec![
            WebField::bot_id(),
            WebField::reason()
        ],
        RPCMethod::BotVoteReset { .. } => vec![
            WebField::bot_id(),
            WebField::reason()
        ],
        RPCMethod::BotVoteResetAll { .. } => vec![
            WebField::reason()
        ],
        RPCMethod::BotUnverify { .. } => vec![
            WebField::bot_id(),
            WebField::reason()
        ],
        RPCMethod::BotPremiumAdd { .. } => vec![
            WebField::bot_id(),
            WebField {
                id: "time_period_hours".to_string(),
                label: "Time [X unit(s)]".to_string(),
                field_type: FieldType::Hour,
                icon: "material-symbols:timer".to_string(),
                placeholder: "Time period. Format: X years/days/hours".to_string(),
            },
            WebField::reason(),
        ],
        RPCMethod::BotPremiumRemove { .. } => vec![
            WebField::bot_id(),
            WebField::reason()
        ],
        RPCMethod::BotVoteBanAdd { .. } => vec![
            WebField::bot_id(),
            WebField::reason(),
        ],
        RPCMethod::BotVoteBanRemove { .. } => vec![
            WebField::bot_id(),
            WebField::reason(),
        ],
        RPCMethod::BotForceRemove { .. } => vec![
            WebField::bot_id(),
            WebField {
                id: "kick".to_string(),
                label: "Kick the bot from the server".to_string(),
                field_type: FieldType::Boolean,
                icon: "fa-solid:sign-out-alt".to_string(),
                placeholder: "Kick the bot from the server".to_string(),
            },
            WebField::reason(),
        ],
        RPCMethod::BotCertifyAdd { .. } => vec![
            WebField::bot_id(),
            WebField::reason(),
        ],
        RPCMethod::BotCertifyRemove { .. } => vec![
            WebField::bot_id(),
            WebField::reason(),
        ],
        RPCMethod::BotVoteCountSet { .. } => vec![
            WebField::bot_id(),
            WebField {
                id: "count".to_string(),
                label: "Vote count".to_string(),
                field_type: FieldType::Number,
                icon: "material-symbols:timer".to_string(),
                placeholder: "Vote count".to_string(),
            },
            WebField::reason(),
        ],
        RPCMethod::BotTransferOwnershipUser { .. } => vec![
            WebField::bot_id(),
            WebField {
                id: "new_owner".to_string(),
                label: "User ID".to_string(),
                field_type: FieldType::Text,
                icon: "material-symbols:timer".to_string(),
                placeholder: "New Owner".to_string(),
            },
            WebField::reason(),
        ],
        RPCMethod::BotTransferOwnershipTeam { .. } => vec![
            WebField::bot_id(),
            WebField {
                id: "new_team".to_string(),
                label: "Team ID".to_string(),
                field_type: FieldType::Text,
                icon: "material-symbols:timer".to_string(),
                placeholder: "New Team".to_string(),
            },
            WebField::reason(),
        ],
        RPCMethod::TeamNameEdit { .. } => vec![
            WebField {
                id: "team_id".to_string(),
                label: "Team ID".to_string(),
                field_type: FieldType::Text,
                icon: "material-symbols:timer".to_string(),
                placeholder: "Team ID".to_string(),
            },
            WebField {
                id: "new_name".to_string(),
                label: "New team name".to_string(),
                field_type: FieldType::Text,
                icon: "material-symbols:timer".to_string(),
                placeholder: "Team name".to_string(),
            },
            WebField::reason(),
        ],
    }
}

#[derive(Serialize, TS)]
#[ts(export, export_to = ".generated/RPCWebAction.ts")]
struct WebAction {
    id: String,
    label: String,
    description: String,
    needed_perms: RPCPerms,
    method_example: RPCMethod,
    fields: Vec<WebField>,
}

#[derive(Deserialize)]
struct WebActionQuery {
    user_id: Option<String>
}

async fn available_actions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WebActionQuery>,
) -> Result<Json<Vec<WebAction>>, RPCResponse> {    
    let (
        owner,
        head,
        admin,
        staff
    ) = if let Some(id) = query.user_id {
        let count = sqlx::query!(
            "SELECT COUNT(*) FROM users WHERE user_id = $1",
            id
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            RPCResponse::Err(e.to_string())
        })?;

        if count.count.unwrap_or_default() == 0 {
            return Err(RPCResponse::UserNotFound);
        }

        let perms = sqlx::query!(
            "SELECT owner, hadmin, iblhdev, admin, staff FROM users WHERE user_id = $1",
            id
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            RPCResponse::Err(e.to_string())
        })?;

        (
            perms.owner,
            perms.hadmin || perms.iblhdev,
            perms.admin,
            perms.staff
        )
    } else {
        (
            true,
            true,
            true,
            true
        )
    };

    let mut actions = Vec::new();

    for variant in super::core::RPCMethod::VARIANTS {
        let method = super::core::RPCMethod::from_str(variant)
        .map_err(|e| {
            RPCResponse::Err(e.to_string())
        })?;

        let action = WebAction {
            id: variant.to_string(),
            label: method.label(),
            description: method.description(),
            needed_perms: method.needs_perms(),
            method_example: method.clone(),
            fields: method_web_fields(method),
        };

        match action.needed_perms {
            RPCPerms::Owner => {
                if owner {
                    actions.push(action);
                }
            },
            RPCPerms::Head => {
                if head {
                    actions.push(action);
                }
            },
            RPCPerms::Admin => {
                if admin {
                    actions.push(action);
                }
            },
            RPCPerms::Staff => {
                if staff {
                    actions.push(action);
                }
            },
        }
    }

    Ok(Json(actions))
}
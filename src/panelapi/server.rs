use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use crate::impls;
use axum::Json;
use axum::http::HeaderMap;

use axum::response::{Response, IntoResponse};
use axum::routing::{post, get};
use axum::{
    extract::State,
    http::StatusCode,
    Router
};
use log::info;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

struct Error {
    status: StatusCode,
    message: String,
}

impl Error {
    fn new(e: impl Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: e.to_string(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}


pub struct AppState {
    pub cache_http: impls::cache::CacheHttpImpl,
    pub pool: PgPool,
}

pub async fn init_panelapi(pool: PgPool, cache_http: impls::cache::CacheHttpImpl) {
    use utoipa::OpenApi;
    #[derive(OpenApi)]
    #[openapi(paths(authenticate), components(schemas()))]
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

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS rpc__panelauthchain (
            user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
            token TEXT NOT NULL UNIQUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    )
    .execute(&pool)
    .await
    .expect("Failed to create rpc__panelauthchain table");

    
    let shared_state = Arc::new(AppState { pool, cache_http });

    let app = Router::new()
        .route("/authorize", post(authenticate))
        .route("/openapi", get(docs))
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

    info!("Starting PanelAPI server on {}", addr);

    if let Err(e) = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
    {
        panic!("PanelAPI server error: {}", e);
    }
}

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = ".generated/LoginOp.ts")]
pub enum LoginOp {
    GetLoginUrl {
        version: u16,
    },
    Login {
        code: String,
    },
}

/// Authenticate User
#[utoipa::path(
    post,
    request_body = RPCRequest,
    path = "/",
    responses(
        (status = 200, description = "Content", body = String),
        (status = 204, description = "No content"),
        (status = BAD_REQUEST, description = "An error occured", body = String),
    ),
)]
#[axum_macros::debug_handler]
async fn authenticate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginOp>,
) -> Result<impl IntoResponse, Error> {
    match req {
        LoginOp::GetLoginUrl { version } => {
            if version != 0 {
                return Ok((StatusCode::BAD_REQUEST, "Invalid version".to_string()));
            }

            Ok(
                (
                    StatusCode::OK, 
                    format!(
                        "https://discord.com/api/oauth2/authorize?client_id={client_id}&redirect_uri={redirect_url}&response_type=code&scope=identify",
                        client_id = crate::config::CONFIG.panel_login.client_id,
                        redirect_url = crate::config::CONFIG.panel_login.redirect_url
                    )
                )
            )
        },
        LoginOp::Login { code } => {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(Error::new)?;

            let resp = client
                .post("https://discord.com/api/oauth2/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("User-Agent", "DiscordBot (arcadia v1.0)")
                .form(&[
                    ("client_id", crate::config::CONFIG.panel_login.client_id.as_str()),
                    ("client_secret", crate::config::CONFIG.panel_login.client_secret.as_str()),
                    ("grant_type", "authorization_code"),
                    ("code", code.as_str()),
                    ("redirect_uri", crate::config::CONFIG.panel_login.redirect_url.as_str()),
                    ("scope", "identify"),
                ])
                .send()
                .await
                .map_err(Error::new)?;
            
            #[derive(Deserialize)]
            struct Oauth2 {
                access_token: String
            }

            let oauth2 = resp.json::<Oauth2>().await.map_err(Error::new)?;

            let user_resp = client
            .get("https://discord.com/api/users/@me")
            .header("Authorization", "Bot ".to_string() + oauth2.access_token.as_str())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", "DiscordBot (arcadia v1.0)")
            .send()
            .await
            .map_err(Error::new)?;

            let user = user_resp.json::<serenity::model::user::User>().await.map_err(Error::new)?;

            let rec = sqlx::query!(
                "SELECT staff FROM users WHERE user_id = $1",
                user.id.to_string()
            )
            .fetch_one(&state.pool)
            .await
            .map_err(Error::new)?;

            if !rec.staff {
                return Ok((StatusCode::FORBIDDEN, "You are not staff".to_string()));
            }

            let token = crate::impls::crypto::gen_random(4196);

            sqlx::query!(
                "INSERT INTO rpc__panelauthchain (user_id, token) VALUES ($1, $2)",
                user.id.to_string(),
                token
            )
            .execute(&state.pool)
            .await
            .map_err(Error::new)?;

            Ok((
                StatusCode::OK, 
                token
            ))
        }
    }
}

/*
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
*/
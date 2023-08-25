use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use crate::impls;
use crate::impls::target_types::TargetType;
use crate::panelapi::types::InstanceConfig;
use axum::Json;
use axum::extract::Host;
use axum::http::HeaderMap;

use axum::response::{Response, IntoResponse};
use axum::routing::{post, get};
use axum::{
    extract::State,
    http::StatusCode,
    Router
};
use log::info;
use serenity::all::User;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use strum_macros::{Display, EnumString, EnumVariantNames};

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
    #[openapi(paths(get_instance_config, query), components(schemas(PanelQuery, InstanceConfig)))]
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
            token TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    )
    .execute(&pool)
    .await
    .expect("Failed to create rpc__panelauthchain table");
    
    let shared_state = Arc::new(AppState { pool, cache_http });

    let app = Router::new()
        .route("/openapi", get(docs))
        .route("/query", post(query))
        .route("/", get(get_instance_config))
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

#[derive(Serialize, Deserialize, ToSchema, TS, EnumString, EnumVariantNames, Display, Clone)]
#[ts(export, export_to = ".generated/PanelQuery.ts")]
pub enum PanelQuery {
    /// Get Login URL
    GetLoginUrl {
        version: u16,
        redirect_url: String
    },
    /// Login, returning a login token
    Login {
        code: String,
        redirect_url: String,
    },
    /// Get Identity (user_id/created_at) for a given login token
    GetIdentity {
        login_token: String,
    },
    /// Returns user information given a user id, returning a dovewing PartialUser
    GetUserDetails {
        user_id: String,
    },
    /// Given a user ID, returns the permissions for that user
    GetUserPerms {
        user_id: String,
    },
    /// Given a login token, returns the capabilities for that user
    GetCapabilities {
        login_token: String,
    },
    /// Given a version, returns core constants for the panel
    GetCoreConstants {
        login_token: String,
    },
    /// Returns the bot queue
    BotQueue {
        login_token: String,
    },
}

/// Make Panel Query
#[utoipa::path(
    post,
    path = "/",
    responses(
        (status = 200, description = "Content", body = InstanceConfig),
        (status = 204, description = "No content"),
        (status = BAD_REQUEST, description = "An error occured", body = String),
    ),
)]
#[axum_macros::debug_handler]
async fn get_instance_config(
    Host(host): Host,
) -> Result<impl IntoResponse, Error> {
    Ok(
        (
            StatusCode::OK, 
            Json(
                super::types::InstanceConfig {
                    description: "Arcadia Production Instance Config".to_string(),
                    instance_url: host,
                    query: "/query".to_string(),
                }
            )
        ).into_response()
    ) 
}

/// Make Panel Query
#[utoipa::path(
    post,
    request_body =  PanelQuery,
    path = "/",
    responses(
        (status = 200, description = "Content", body = String),
        (status = 204, description = "No content"),
        (status = BAD_REQUEST, description = "An error occured", body = String),
    ),
)]
#[axum_macros::debug_handler]
async fn query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PanelQuery>,
) -> Result<impl IntoResponse, Error> {
    match req {
        PanelQuery::GetLoginUrl { version, redirect_url } => {
            if version != 0 {
                return Ok((StatusCode::BAD_REQUEST, "Invalid version".to_string()).into_response());
            }

            Ok(
                (
                    StatusCode::OK, 
                    format!(
                        "https://discord.com/api/oauth2/authorize?client_id={client_id}&redirect_uri={redirect_url}&response_type=code&scope=identify",
                        client_id = crate::config::CONFIG.panel_login.client_id,
                        redirect_url = redirect_url
                    )
                ).into_response()
            )
        },
        PanelQuery::Login { code, redirect_url } => {
            if !crate::config::CONFIG.panel_login.redirect_url.contains(&redirect_url) {
                return Ok((StatusCode::BAD_REQUEST, "Invalid redirect url".to_string()).into_response());
            }

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
                    ("redirect_uri", redirect_url.as_str()),
                    ("scope", "identify"),
                ])
                .send()
                .await
                .map_err(Error::new)?
                .error_for_status()
                .map_err(Error::new)?;
            
            #[derive(Deserialize)]
            struct Oauth2 {
                access_token: String
            }

            let oauth2 = resp.json::<Oauth2>().await.map_err(Error::new)?;

            let user_resp = client
            .get("https://discord.com/api/users/@me")
            .header("Authorization", "Bearer ".to_string() + oauth2.access_token.as_str())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", "DiscordBot (arcadia v1.0)")
            .send()
            .await
            .map_err(Error::new)?
            .error_for_status()
            .map_err(Error::new)?;

            let user = user_resp.json::<User>().await.map_err(Error::new)?;

            let rec = sqlx::query!(
                "SELECT staff FROM users WHERE user_id = $1",
                user.id.to_string()
            )
            .fetch_one(&state.pool)
            .await
            .map_err(Error::new)?;

            if !rec.staff {
                return Ok((StatusCode::FORBIDDEN, "You are not staff".to_string()).into_response());
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
            ).into_response())
        }
        PanelQuery::GetIdentity { login_token } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token).await.map_err(Error::new)?;

            Ok(
                (
                    StatusCode::OK, 
                    Json(auth_data)
                ).into_response()
            )
        },
        PanelQuery::GetUserDetails { user_id } => {
            let user = crate::impls::dovewing::get_partial_user(&state.pool, &user_id).await.map_err(Error::new)?;

            Ok(
                (
                    StatusCode::OK, 
                    Json(user)
                ).into_response()
            )
        },
        PanelQuery::GetUserPerms { user_id } => {
            let perms = super::auth::get_user_perms(&state.pool, &user_id).await.map_err(Error::new)?;

            Ok(
                (
                    StatusCode::OK, 
                    Json(perms)
                ).into_response()
            )
        },
        PanelQuery::GetCapabilities { login_token } => {
            let caps = super::auth::get_capabilities(&state.pool, &login_token).await.map_err(Error::new)?;

            Ok(
                (
                    StatusCode::OK, 
                    Json(caps)
                ).into_response()
            )
        },
        PanelQuery::GetCoreConstants { login_token } => {
            // Ensure auth is valid, that's all that matters here
            super::auth::check_auth(&state.pool, &login_token).await.map_err(Error::new)?;

            Ok(
                (
                    StatusCode::OK, 
                    Json(
                        super::types::CoreConstants {
                            frontend_url: crate::config::CONFIG.frontend_url.clone(),
                            infernoplex_url: crate::config::CONFIG.infernoplex_url.clone(),
                        }
                    )
                ).into_response()
            )
        },
        PanelQuery::BotQueue { login_token } => {
            super::auth::check_auth(&state.pool, &login_token).await.map_err(Error::new)?;

            let queue = sqlx::query!(
                "SELECT bot_id, claimed_by, approval_note, short, invite FROM bots WHERE type = 'pending' OR type = 'claimed' ORDER BY created_at"
            )
            .fetch_all(&state.pool)
            .await
            .map_err(Error::new)?;

            let mut bots = Vec::new();

            for bot in queue {
                let owners = crate::impls::utils::get_entity_managers(TargetType::Bot, &bot.bot_id, &state.pool).await.map_err(Error::new)?;

                let user = crate::impls::dovewing::get_partial_user(&state.pool, &bot.bot_id).await.map_err(Error::new)?;

                bots.push(
                    super::types::QueueBot {
                        bot_id: bot.bot_id,
                        user,
                        claimed_by: bot.claimed_by,
                        approval_note: bot.approval_note,
                        short: bot.short,
                        mentionable: owners.mentionables(),
                        invite: bot.invite,
                    }
                );
            }

            Ok(
                (
                    StatusCode::OK, 
                    Json(bots)
                ).into_response()
            )
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
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::impls;
use crate::impls::target_types::TargetType;
use crate::panelapi::types::InstanceConfig;
use crate::rpc::core::{RPCMethod, RPCHandle};
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
use rand::Rng;
use serenity::all::User;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use strum_macros::{Display, EnumVariantNames, EnumString};
use strum::VariantNames;

//use std::time::{SystemTime, UNIX_EPOCH};

// The default time step used by this module internally
//const THOTP_TIME_STEP: u8 = 30;

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
    #[openapi(paths(get_instance_config, query), components(schemas(PanelQuery, InstanceConfig, RPCMethod, TargetType)))]
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
        "CREATE TABLE IF NOT EXISTS staffpanel__authchain (
            itag UUID NOT NULL UNIQUE DEFAULT uuid_generate_v4(),
            paneldata_ref UUID NOT NULL REFERENCES staffpanel__paneldata(itag) ON DELETE CASCADE,
            user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
            token TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            state TEXT NOT NULL DEFAULT 'pending'
        )"
    )
    .execute(&pool)
    .await
    .expect("Failed to create staffpanel__authchain table");

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS staffpanel__paneldata (
            itag UUID NOT NULL UNIQUE DEFAULT uuid_generate_v4(),
            user_id TEXT PRIMARY KEY REFERENCES users(user_id) ON DELETE CASCADE,
            mfa_secret TEXT NOT NULL,
            mfa_verified BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    )
    .execute(&pool)
    .await
    .expect("Failed to create staffpanel__paneldata table");
    
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

#[derive(Serialize, Deserialize, ToSchema, TS, Display, Clone, EnumString, EnumVariantNames)]
#[ts(export, export_to = ".generated/PanelQuery.ts")]
pub enum PanelQuery {
    /// Get Login URL
    GetLoginUrl {
        /// Panel protocol version
        version: u16,
        /// Redirect URL
        redirect_url: String
    },
    /// Login, returning a login token
    Login {
        /// Discord OAuth2 code
        code: String,
        /// Redirect URL
        redirect_url: String,
    },
    /// Check MFA status for a given login token
    LoginMfaCheckStatus {
        /// Login token
        login_token: String,
    },
    /// Activates a session for a given login token
    LoginActivateSession {
        /// Login token
        login_token: String,
        /// MFA code
        otp: String,
    },
    /// Resets MFA for a user identified by login token
    LoginResetMfa {
        /// Login token
        login_token: String,
        /// Old MFA code
        otp: String,
    },
    /// Logs out a session. Should be called when the user logs out of the panel
    Logout {
        /// Login token
        login_token: String,
    },
    /// Get Identity (user_id/created_at) for a given login token
    GetIdentity {
        /// Login token
        login_token: String,
    },
    /// Returns user information given a user id, returning a dovewing PartialUser
    GetUserDetails {
        /// User ID to fetch details for
        user_id: String,
    },
    /// Given a user ID, returns the permissions for that user
    GetUserPerms {
        /// User ID to fetch perms for
        user_id: String,
    },
    /// Given a login token, returns the capabilities for that user
    GetCapabilities {
        /// Login token
        login_token: String,
    },
    /// Given a login token, returns core constants for the panel for that user
    GetCoreConstants {
        /// Login token
        login_token: String,
    },
    /// Returns the bot queue
    BotQueue {
        /// Login token
        login_token: String,
    },
    /// Executes an RPC on a target
    ExecuteRpc {
        /// Login token
        login_token: String,
        /// Target Type
        target_type: TargetType,
        /// RPC Method
        method: RPCMethod
    },
    /// Returns all RPC actions available
    /// 
    /// Setting filtered will filter RPC actions to that what the user has access to
    GetRpcMethods {
        /// Login token
        login_token: String,
        /// Filtered
        filtered: bool,
    }
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
    request_body = PanelQuery,
    path = "/",
    responses(
        (status = 200, description = "Content", body = String),
        (status = 204, description = "No content"),
        (status = BAD_REQUEST, description = "An error occured", body = String),
    ),
)]
//#[axum_macros::debug_handler]
async fn query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PanelQuery>,
) -> Result<impl IntoResponse, Error> {
    match req {
        PanelQuery::GetLoginUrl { version, redirect_url } => {
            if version != 1 {
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

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            sqlx::query!(
                "DELETE FROM staffpanel__authchain WHERE user_id = $1",
                user.id.to_string()
            )
            .execute(&mut tx)
            .await
            .map_err(Error::new)?;

            // Create a random number between 4196 and 8192 for the token
            let tlength = rand::thread_rng().gen_range(4196..8192);

            let token = crate::impls::crypto::gen_random(tlength as usize);

            let count = sqlx::query!(
                "SELECT COUNT(*) FROM staffpanel__paneldata WHERE user_id = $1",
                user.id.to_string()
            )
            .fetch_one(&mut tx)
            .await
            .map_err(Error::new)?
            .count
            .unwrap_or(0);

            let itag = if count == 0 {
                let temp_secret = thotp::generate_secret(160);

                let temp_secret_enc = thotp::encoding::encode(&temp_secret, data_encoding::BASE32);

                sqlx::query!(
                    "INSERT INTO staffpanel__paneldata (user_id, mfa_secret) VALUES ($1, $2) RETURNING itag",
                    user.id.to_string(),
                    temp_secret_enc
                )
                .fetch_one(&mut tx)
                .await
                .map_err(Error::new)?
                .itag
            } else {
                sqlx::query!(
                    "SELECT itag FROM staffpanel__paneldata WHERE user_id = $1",
                    user.id.to_string()
                )
                .fetch_one(&mut tx)
                .await
                .map_err(Error::new)?
                .itag
            };

            sqlx::query!(
                "INSERT INTO staffpanel__authchain (user_id, paneldata_ref, token) VALUES ($1, $2, $3)",
                user.id.to_string(),
                itag,
                token,
            )
            .execute(&mut tx)
            .await
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            // Stage 1 of login is done, panel will handle MFA next
            Ok((
                StatusCode::OK, 
                token
            ).into_response())
        },
        PanelQuery::LoginMfaCheckStatus { login_token } => {
            let auth_data = super::auth::check_auth_insecure(&state.pool, &login_token).await.map_err(Error::new)?;
            if auth_data.state != "pending" {
                return Err(
                    Error {
                        status: StatusCode::BAD_REQUEST,
                        message: "sessionAlreadyActive".to_string(),
                    }
                )
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            // Check if user already has MFA setup
            let count = sqlx::query!(
                "SELECT COUNT(*) FROM staffpanel__paneldata WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_one(&mut tx)
            .await
            .map_err(Error::new)?
            .count
            .unwrap_or(0);

            if count == 0 {
                // This should never happen, as Login creates a dummy MFA setup
                return Err(
                    Error {
                        status: StatusCode::BAD_REQUEST,
                        message: "invalidPanelData".to_string(),
                    }
                );
            }
            
            // Check if user has MFA setup
            let mrec = sqlx::query!(
                "SELECT mfa_verified FROM staffpanel__paneldata WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_one(&mut tx)
            .await
            .map_err(Error::new)?;

            if !mrec.mfa_verified {
                // User does not have MFA setup, generate a secret
                let secret_vec = thotp::generate_secret(160);
                let secret = thotp::encoding::encode(&secret_vec, data_encoding::BASE32);

                sqlx::query!(
                    "UPDATE staffpanel__paneldata SET mfa_secret = $2 WHERE user_id = $1",
                    auth_data.user_id,
                    secret
                )
                .execute(&mut tx)
                .await
                .map_err(Error::new)?;

                let qr_code_uri = thotp::qr::otp_uri(
                    // Type of otp
                    "totp",
                    // The encoded secret
                    &secret,
                    // Your big corp title
                    "Infinity Bot List:staff@infinitybots.gg",
                    // Your big corp issuer
                    "Infinity Bot List",
                    // The counter (Only HOTP)
                    None,
                )
                .map_err(Error::new)?;    

                let qr = thotp::qr::generate_code_svg(
                    &qr_code_uri,
                    // The qr code width (None defaults to 200)
                    None,
                    // The qr code height (None defaults to 200)
                    None,
                    // Correction level, M is the default
                    thotp::qr::EcLevel::M,
                )        
                .map_err(Error::new)?;  

                tx.commit().await.map_err(Error::new)?;   

                Ok((
                    StatusCode::OK, 
                    Json(
                        super::types::MfaLogin {
                            info: Some(super::types::MfaLoginSecret {
                                qr_code: qr,
                                otp_url: qr_code_uri,
                                secret,
                            }),
                        }
                    )
                ).into_response())
            } else {
                tx.rollback().await.map_err(Error::new)?;

                Ok((
                    StatusCode::OK, 
                    Json(
                        super::types::MfaLogin {
                            info: None,
                        }
                    )
                ).into_response())
            }
        },
        PanelQuery::LoginActivateSession { login_token, otp } => {
            let auth_data = super::auth::check_auth_insecure(&state.pool, &login_token).await.map_err(Error::new)?;

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            let count = sqlx::query!(
                "SELECT COUNT(*) FROM staffpanel__paneldata WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_one(&mut tx)
            .await
            .map_err(Error::new)?
            .count
            .unwrap_or(0);     

            if count == 0 {
                return Err(
                    Error {
                        status: StatusCode::BAD_REQUEST,
                        message: "mfaNotSetup".to_string(),
                    }
                )
            }      

            let secret = sqlx::query!(
                "SELECT mfa_secret FROM staffpanel__paneldata WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_one(&mut tx)
            .await
            .map_err(Error::new)?
            .mfa_secret;
            
            let secret = thotp::encoding::decode(&secret, data_encoding::BASE32).map_err(Error::new)?;

            let (result, _discrepancy) = thotp::verify_totp(&otp, &secret, 0).unwrap();

            if !result {
                return Err(
                    Error {
                        status: StatusCode::BAD_REQUEST,
                        message: "mfaInvalidCode".to_string(),
                    }
                )
            }

            sqlx::query!(
                "UPDATE staffpanel__authchain SET state = 'active' WHERE token = $1",
                login_token
            )
            .execute(&mut tx)
            .await
            .map_err(Error::new)?;

            sqlx::query!(
                "UPDATE staffpanel__paneldata SET mfa_verified = TRUE WHERE user_id = $1",
                auth_data.user_id
            )
            .execute(&mut tx)
            .await
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((
                StatusCode::NO_CONTENT, 
                ""
            ).into_response())
        },
        PanelQuery::LoginResetMfa { login_token, otp } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token).await.map_err(Error::new)?;

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            let count = sqlx::query!(
                "SELECT COUNT(*) FROM staffpanel__paneldata WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_one(&mut tx)
            .await
            .map_err(Error::new)?
            .count
            .unwrap_or(0);     

            if count == 0 {
                return Err(
                    Error {
                        status: StatusCode::BAD_REQUEST,
                        message: "mfaNotSetup".to_string(),
                    }
                )
            } 

            let secret = sqlx::query!(
                "SELECT mfa_secret FROM staffpanel__paneldata WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_one(&mut tx)
            .await
            .map_err(Error::new)?
            .mfa_secret;
            
            let secret = thotp::encoding::decode(&secret, data_encoding::BASE32).map_err(Error::new)?;

            let (result, _discrepancy) = thotp::verify_totp(&otp, &secret, 0).unwrap();

            if !result {
                return Err(
                    Error {
                        status: StatusCode::BAD_REQUEST,
                        message: "mfaInvalidCode".to_string(),
                    }
                )
            }

            sqlx::query!(
                "UPDATE staffpanel__paneldata SET mfa_verified = FALSE WHERE user_id = $1",
                auth_data.user_id
            )
            .execute(&mut tx)
            .await
            .map_err(Error::new)?;

            // Revoke existing session
            sqlx::query!(
                "DELETE FROM staffpanel__authchain WHERE user_id = $1",
                auth_data.user_id
            )
            .execute(&mut tx)
            .await
            .map_err(Error::new)?;

            Ok(
                (
                    StatusCode::NO_CONTENT, 
                    ""
                ).into_response()
            )
        },
        PanelQuery::Logout { login_token } => {
            // Just delete the auth, no point in even erroring if it doesn't exist
            let row = sqlx::query!(
                "DELETE FROM staffpanel__authchain WHERE token = $1",
                login_token
            )
            .execute(&state.pool)
            .await
            .map_err(Error::new)?;

            Ok(
                (
                    StatusCode::OK,
                    row.rows_affected().to_string()
                ).into_response()
            )
        },
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
                            servers: super::types::PanelServers {
                                main: crate::config::CONFIG.servers.main.to_string(),
                                staff: crate::config::CONFIG.servers.staff.to_string(),
                                testing: crate::config::CONFIG.servers.testing.to_string(),
                            }
                        }
                    )
                ).into_response()
            )
        },
        PanelQuery::BotQueue { login_token } => {
            let caps = super::auth::get_capabilities(&state.pool, &login_token).await.map_err(Error::new)?;

            if !caps.contains(&super::types::Capability::ViewBotQueue) {
                return Ok((StatusCode::FORBIDDEN, "You do not have permission to access the bot queue right now".to_string()).into_response());
            }

            let queue = sqlx::query!(
                "SELECT bot_id, client_id, claimed_by, approval_note, short, invite FROM bots WHERE type = 'pending' OR type = 'claimed' ORDER BY created_at"
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
                        client_id: bot.client_id,
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
        },
        PanelQuery::ExecuteRpc { login_token, target_type, method } => {
            let caps = super::auth::get_capabilities(&state.pool, &login_token).await.map_err(Error::new)?;

            if !caps.contains(&super::types::Capability::Rpc) {
                return Ok((StatusCode::FORBIDDEN, "You do not have permission to use RPC right now".to_string()).into_response());
            }

            let auth_data = super::auth::check_auth(&state.pool, &login_token).await.map_err(Error::new)?;

            let resp = method.handle(
                RPCHandle {
                    pool: state.pool.clone(),
                    cache_http: state.cache_http.clone(),
                    user_id: auth_data.user_id,
                    target_type,
                }
            )
            .await;

            match resp {
                Ok(r) => match r {
                    crate::rpc::core::RPCSuccess::NoContent => {
                        Ok((
                            StatusCode::NO_CONTENT, 
                            ""
                        ).into_response())
                    },
                    crate::rpc::core::RPCSuccess::Content(c) => {
                        Ok((
                            StatusCode::OK, 
                            c
                        ).into_response())
                    }
                },
                Err(e) => {
                    Ok((
                        StatusCode::BAD_REQUEST, 
                        e.to_string()
                    ).into_response())
                }
            }
        },
        PanelQuery::GetRpcMethods { login_token, filtered } => {
            let caps = super::auth::get_capabilities(&state.pool, &login_token).await.map_err(Error::new)?;

            if !caps.contains(&super::types::Capability::Rpc) {
                return Ok((StatusCode::FORBIDDEN, "You do not have permission to use RPC right now".to_string()).into_response());
            }

            let auth_data = super::auth::check_auth(&state.pool, &login_token).await.map_err(Error::new)?;

            let (owner, head, admin, staff) = {
                let perms = sqlx::query!(
                    "SELECT owner, hadmin, iblhdev, admin, staff FROM users WHERE user_id = $1",
                    auth_data.user_id
                )
                .fetch_one(&state.pool)
                .await
                .map_err(Error::new)?;

                (
                    perms.owner,
                    perms.hadmin || perms.iblhdev,
                    perms.admin,
                    perms.staff,
                )
            };

            let mut rpc_methods = Vec::new();

            for method in crate::rpc::core::RPCMethod::VARIANTS {
                let variant = crate::rpc::core::RPCMethod::from_str(method).map_err(Error::new)?;

                if filtered {
                    match variant.needs_perms() {
                        crate::rpc::core::RPCPerms::Owner => {
                            if !owner {
                                continue;
                            }
                        }
                        crate::rpc::core::RPCPerms::Head => {
                            if !head {
                                continue;
                            }
                        }
                        crate::rpc::core::RPCPerms::Admin => {
                            if !admin {
                                continue;
                            }
                        }
                        crate::rpc::core::RPCPerms::Staff => {
                            if !staff {
                                continue;
                            }
                        }
                    }
                }

                let action = super::types::RPCWebAction {
                    id: method.to_string(),
                    label: variant.label(),
                    description: variant.description(),
                    needs_perms: variant.needs_perms(),
                    supported_target_types: variant.supported_target_types(),
                    fields: variant.method_fields(),
                };

                rpc_methods.push(action);
            }        

            Ok((
                StatusCode::OK, 
                Json(rpc_methods)
            ).into_response())
        }
    }
}

use std::fmt::Display;
use std::os::unix::prelude::PermissionsExt;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::impls::link::Link;
use crate::impls::{target_types::TargetType, utils::get_user_perms};
use crate::panelapi::types::staff_disciplinary::StaffDisciplinaryType;
use crate::panelapi::types::webcore::{Hello, StartAuth};
use crate::panelapi::types::{
    analytics::BaseAnalytics,
    auth::{AuthorizeAction, MfaLogin, MfaLoginSecret},
    blog::{BlogAction, BlogPost},
    bot_whitelist::{BotWhitelist, BotWhitelistAction},
    cdn::{CdnAssetAction, CdnAssetItem},
    changelogs::{ChangelogAction, ChangelogEntry},
    entity::{PartialBot, PartialEntity, PartialServer},
    partners::{CreatePartner, Partner, PartnerAction, PartnerType, Partners},
    rpc::RPCWebAction,
    rpclogs::RPCLogEntry,
    shop_items::{ShopItem, ShopItemAction},
    staff_disciplinary::StaffDisciplinaryTypeAction,
    staff_positions::StaffPosition,
    vote_credit_tiers::{VoteCreditTier, VoteCreditTierAction},
    webcore::{CoreConstants, InstanceConfig, PanelServers},
};
use crate::rpc::core::{RPCHandle, RPCMethod};
use axum::body::StreamBody;
use axum::extract::DefaultBodyLimit;
use axum::http::HeaderMap;
use axum::Json;
use kittycat::perms::{self, Permission};

use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{extract::State, http::StatusCode, Router};
use log::info;
use moka::future::Cache;
use rand::Rng;
use serenity::all::{RoleId, User};
use sqlx::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_http::cors::{Any, CorsLayer};

use super::types::staff_members::StaffMemberAction;
use super::types::staff_positions::{CorrespondingServer, StaffPositionAction};
use crate::impls::dovewing::DovewingSource;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use strum::VariantNames;
use strum_macros::{Display, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;

use num_traits::ToPrimitive;

const HELLO_VERSION: u16 = 5;
const AUTH_VERSION: u16 = 5;

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
    pub cache_http: botox::cache::CacheHttpImpl,
    pub pool: PgPool,
    pub cdn_file_chunks_cache: Cache<String, Vec<u8>>,
}

pub async fn init_panelapi(pool: PgPool, cache_http: botox::cache::CacheHttpImpl) {
    use utoipa::OpenApi;
    #[derive(OpenApi)]
    #[openapi(
        paths(query),
        components(schemas(
            PanelQuery,
            InstanceConfig,
            RPCMethod,
            TargetType,
            CdnAssetAction,
            PartnerAction,
            CreatePartner,
            AuthorizeAction,
            ChangelogAction,
            BlogAction,
            StaffPositionAction,
            StaffMemberAction,
            StaffDisciplinaryTypeAction,
            VoteCreditTierAction,
            ShopItem,
            ShopItemAction,
            BotWhitelistAction,
            Link,
        ))
    )]
    struct ApiDoc;

    async fn docs() -> impl IntoResponse {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        let data = ApiDoc::openapi().to_json();

        if let Ok(data) = data {
            return (headers, data).into_response();
        }

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate docs".to_string(),
        )
            .into_response()
    }

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS staffpanel__authchain (
            itag UUID NOT NULL UNIQUE DEFAULT uuid_generate_v4(),
            user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
            token TEXT NOT NULL,
            popplio_token TEXT NOT NULL, -- The popplio_token is sent to Popplio etc. to validate such requests. It is not visible or disclosed to the client
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            state TEXT NOT NULL DEFAULT 'pending'
        )"
    )
    .execute(&pool)
    .await
    .expect("Failed to create staffpanel__authchain table");

    let cdn_file_chunks_cache = Cache::<String, Vec<u8>>::builder()
        .time_to_live(Duration::from_secs(3600))
        .build();

    let shared_state = Arc::new(AppState {
        pool,
        cache_http,
        cdn_file_chunks_cache,
    });

    let app = Router::new()
        .route("/openapi", get(docs))
        .route("/", post(query))
        .with_state(shared_state)
        .layer(DefaultBodyLimit::max(1048576000))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = format!("127.0.0.1:{}", crate::config::CONFIG.server_port.get())
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

#[derive(Serialize, Deserialize, ToSchema, TS, Display, Clone, EnumVariantNames)]
#[ts(export, export_to = ".generated/PanelQuery.ts")]
pub enum PanelQuery {
    /// Authorization-related commands
    Authorize {
        /// Authorize protocol version, should be `AUTH_VERSION`
        version: u16,
        /// Action to take
        action: AuthorizeAction,
    },
    /// Returns configuration data for the panel
    Hello {
        /// Login token
        login_token: String,
        /// Hello protocol version, should be `HELLO_VERSION`
        version: u16,
    },
    /// Returns base analytics
    BaseAnalytics {
        /// Login token
        login_token: String,
    },
    /// Returns user information given a user id, returning a dovewing PartialUser
    GetUser {
        /// Login token
        login_token: String,
        /// User ID to fetch details for
        user_id: String,
    },
    /// Returns the bot queue
    ///
    /// This is public to all staff members
    BotQueue {
        /// Login token
        login_token: String,
    },
    /// Executes an RPC on a target
    ///
    /// The endpoint itself is public to all staff members however RPC will only execute if the user has permission for the RPC method
    ExecuteRpc {
        /// Login token
        login_token: String,
        /// Target Type
        target_type: TargetType,
        /// RPC Method
        method: RPCMethod,
    },
    /// Returns all RPC actions available
    ///
    /// Setting filtered will filter RPC actions to that what the user has access to
    ///
    /// This is public to all staff members
    GetRpcMethods {
        /// Login token
        login_token: String,
        /// Filtered
        filtered: bool,
    },
    /// Gets the list of all RPC log entries made
    GetRpcLogEntries {
        /// Login token
        login_token: String,
    },
    /// Searches for a bot based on a query
    ///
    /// This is public to all staff members
    SearchEntitys {
        /// Login token
        login_token: String,
        /// Target type
        target_type: TargetType,
        /// Query
        query: String,
    },
    /// Uploads a chunk of data returning a chunk ID
    ///
    /// Chunks expire after 10 minutes and are stored in memory
    ///
    /// After uploading all chunks for a file, use `AddFile` to create the file
    ///
    /// Needs `cdn.upload_chunk` permission
    UploadCdnFileChunk {
        /// Login token
        login_token: String,
        /// Array of bytes of the chunk contents
        chunk: Vec<u8>,
    },
    /// Lists all available CDN scopes
    ///
    /// Needs `cdn.list_scopes` permission
    ListCdnScopes {
        /// Login token
        login_token: String,
    },
    /// Returns the main CDN scope for Infinity Bot List
    ///
    /// This is public to all staff members
    GetMainCdnScope {
        /// Login token
        login_token: String,
    },
    /// Updates/handles an asset on the CDN
    ///
    /// Needs `cdn.update_asset` permission. Not yet granular/action specific
    UpdateCdnAsset {
        /// Login token
        login_token: String,
        /// CDN scope
        ///
        /// This describes a location where the CDN may be stored on disk and should be a full path to it
        ///
        /// Currently the panel uses the following scopes:
        ///
        /// `ibl@main`
        cdn_scope: String,
        /// Asset name
        name: String,
        /// Path
        path: String,
        /// Action to take
        action: CdnAssetAction,
    },
    /// Updates/handles partners
    UpdatePartners {
        /// Login token
        login_token: String,
        /// Action
        action: PartnerAction,
    },
    /// Updates/handles the changelog of the list
    UpdateChangelog {
        /// Login token
        login_token: String,
        /// Action
        action: ChangelogAction,
    },
    /// Updates/handles the blog of the list
    UpdateBlog {
        /// Login token
        login_token: String,
        /// Action
        action: BlogAction,
    },
    /// Fetch and modify staff positions
    UpdateStaffPositions {
        /// Login token
        login_token: String,
        /// Action
        action: StaffPositionAction,
    },
    /// Fetch and modify staff members
    UpdateStaffMembers {
        /// Login token
        login_token: String,
        /// Action
        action: StaffMemberAction,
    },
    /// Fetch and update staff disciplinary types
    UpdateStaffDisciplinaryType {
        /// Login token
        login_token: String,
        /// Action
        action: StaffDisciplinaryTypeAction,
    },
    /// Fetch and update/modify vote credit tiers
    UpdateVoteCreditTiers {
        /// Login token
        login_token: String,
        /// Action
        action: VoteCreditTierAction,
    },
    /// Fetch and update/modify shop items
    UpdateShopItems {
        /// Login token
        login_token: String,
        /// Action
        action: ShopItemAction,
    },
    /// Fetch and update/modify bot whitelist
    UpdateBotWhitelist {
        /// Login token
        login_token: String,
        /// Action
        action: BotWhitelistAction,
    },
    /// Create a request to a/an Popplio staff endpoint
    PopplioStaff {
        /// Login token
        login_token: String,
        /// Path
        path: String,
        /// Method
        method: String,
        /// Body
        body: String,
    },
}

/// CDN granularity: Check for [cdn].[permission] or [cdn#scope].[permission]
fn has_cdn_perm(user_perms: &[Permission], cdn_scope: &str, perm: &str) -> bool {
    perms::has_perm(user_perms, &format!("cdn#{}.{}", cdn_scope, perm).into())
        || perms::has_perm(user_perms, &format!("cdn.{}", perm).into())
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
#[axum_macros::debug_handler]
async fn query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PanelQuery>,
) -> Result<impl IntoResponse, Error> {
    match req {
        PanelQuery::Authorize { action, version } => {
            if version != AUTH_VERSION {
                return Ok((StatusCode::BAD_REQUEST, "Invalid version".to_string()).into_response());
            }

            match action {
                AuthorizeAction::Begin {
                    scope,
                    redirect_url,
                } => {
                    if scope != crate::config::CONFIG.panel.panel_scope {
                        return Ok(
                            (StatusCode::BAD_REQUEST, "Invalid scope".to_string()).into_response()
                        );
                    }

                    Ok(
                        (
                            StatusCode::OK,
                            Json(
                                StartAuth {
                                    login_url: format!(
                                        "https://discord.com/api/oauth2/authorize?client_id={client_id}&redirect_uri={redirect_url}&response_type=code&scope=identify",
                                        client_id = crate::config::CONFIG.panel.client_id,
                                        redirect_url = redirect_url
                                    ),
                                    scope: crate::config::CONFIG.panel.panel_scope.clone(),
                                    response_scope: crate::config::CONFIG.panel.panel_response_scope.clone(),
                                }
                            )
                        ).into_response()
                    )
                }
                AuthorizeAction::CreateSession { code, redirect_url } => {
                    if !crate::config::CONFIG
                        .panel
                        .redirect_url
                        .contains(&redirect_url)
                    {
                        return Ok(
                            (StatusCode::BAD_REQUEST, "Invalid redirect url".to_string())
                                .into_response(),
                        );
                    }

                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(10))
                        .build()
                        .map_err(Error::new)?;

                    let resp = client
                        .post("https://discord.com/api/oauth2/token")
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .header("User-Agent", "DiscordBot (arcadia v1.0)")
                        .form(&[
                            ("client_id", crate::config::CONFIG.panel.client_id.as_str()),
                            (
                                "client_secret",
                                crate::config::CONFIG.panel.client_secret.as_str(),
                            ),
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
                        access_token: String,
                    }

                    let oauth2 = resp.json::<Oauth2>().await.map_err(Error::new)?;

                    let user_resp = client
                        .get("https://discord.com/api/users/@me")
                        .header(
                            "Authorization",
                            "Bearer ".to_string() + oauth2.access_token.as_str(),
                        )
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .header("User-Agent", "DiscordBot (arcadia v1.0)")
                        .send()
                        .await
                        .map_err(Error::new)?
                        .error_for_status()
                        .map_err(Error::new)?;

                    let user = user_resp.json::<User>().await.map_err(Error::new)?;

                    let rec = sqlx::query!(
                        "SELECT positions FROM staff_members WHERE user_id = $1",
                        user.id.to_string()
                    )
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let Some(positions) = rec else {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You are not a staff member [not in db]".to_string(),
                        )
                            .into_response());
                    };

                    if positions.positions.is_empty() {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You are not a staff member [no positions]".to_string(),
                        )
                            .into_response());
                    }

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    sqlx::query!(
                        "DELETE FROM staffpanel__authchain WHERE user_id = $1",
                        user.id.to_string()
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    // Create a random number between 4196 and 6000 for the token
                    let tlength = rand::thread_rng().gen_range(4196..6000);

                    let token = botox::crypto::gen_random(tlength as usize);

                    sqlx::query!(
                        "INSERT INTO staffpanel__authchain (user_id, token, popplio_token, state) VALUES ($1, $2, $3, $4)",
                        user.id.to_string(),
                        token,
                        botox::crypto::gen_random(2048),
                        "pending"
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::OK, token).into_response())
                }
                AuthorizeAction::CheckMfaState { login_token } => {
                    let auth_data = super::auth::check_auth_insecure(&state.pool, &login_token)
                        .await
                        .map_err(Error::new)?;

                    if auth_data.state != "pending" && auth_data.state != "active" {
                        return Err(Error {
                            status: StatusCode::BAD_REQUEST,
                            message:
                                "This endpoint can only be used by pending and active sessions"
                                    .to_string(),
                        });
                    }

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    let mfa = sqlx::query!(
                        "SELECT mfa_secret, mfa_verified FROM staff_members WHERE user_id = $1",
                        auth_data.user_id
                    )
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| {
                        Error::new(format!(
                            "Failed to fetch staff member mfa_secret/mfa_verified: {}",
                            e
                        ))
                    })?;

                    if mfa.is_none() {
                        return Err(Error {
                            status: StatusCode::BAD_REQUEST,
                            message: "You are not a staff member".to_string(),
                        });
                    }

                    let mfa = mfa.unwrap();

                    if mfa.mfa_secret.is_none() || !mfa.mfa_verified {
                        let temp_secret = thotp::generate_secret(160);

                        let temp_secret_enc =
                            thotp::encoding::encode(&temp_secret, data_encoding::BASE32);

                        sqlx::query!(
                            "UPDATE staff_members SET mfa_secret = $1 WHERE user_id = $2",
                            &temp_secret_enc,
                            auth_data.user_id,
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(Error::new)?;

                        let qr_code_uri = thotp::qr::otp_uri(
                            // Type of otp
                            "totp",
                            // The encoded secret
                            &temp_secret_enc,
                            // Your big corp title
                            "staff@infinitybots.gg",
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
                            Json(MfaLogin {
                                info: Some(MfaLoginSecret {
                                    qr_code: qr,
                                    otp_url: qr_code_uri,
                                    secret: temp_secret_enc,
                                }),
                            }),
                        )
                            .into_response())
                    } else {
                        tx.rollback().await.map_err(Error::new)?;

                        Ok((StatusCode::OK, Json(MfaLogin { info: None })).into_response())
                    }
                }
                AuthorizeAction::ResetMfaTotp { login_token, otp } => {
                    let auth_data = super::auth::check_auth(&state.pool, &login_token)
                        .await
                        .map_err(Error::new)?;

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    let secret = sqlx::query!(
                        "SELECT mfa_secret FROM staff_members WHERE user_id = $1",
                        auth_data.user_id
                    )
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(Error::new)?
                    .mfa_secret;

                    if secret.is_none() {
                        return Err(Error {
                            status: StatusCode::BAD_REQUEST,
                            message: "mfaNotSetup".to_string(),
                        });
                    }

                    let secret = thotp::encoding::decode(&secret.unwrap(), data_encoding::BASE32)
                        .map_err(Error::new)?;

                    let (result, _discrepancy) = thotp::verify_totp(&otp, &secret, 0).unwrap();

                    if !result {
                        return Err(Error {
                            status: StatusCode::BAD_REQUEST,
                            message: "Invalid OTP Entered".to_string(),
                        });
                    }

                    sqlx::query!(
                        "UPDATE staff_members SET mfa_secret = NULL, mfa_verified = FALSE WHERE user_id = $1",
                        auth_data.user_id
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    // Revoke existing sessions
                    sqlx::query!(
                        "DELETE FROM staffpanel__authchain WHERE user_id = $1",
                        auth_data.user_id
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                AuthorizeAction::ActivateSession { login_token, otp } => {
                    let auth_data = super::auth::check_auth_insecure(&state.pool, &login_token)
                        .await
                        .map_err(Error::new)?;

                    if auth_data.state != "pending" {
                        return Err(Error {
                            status: StatusCode::BAD_REQUEST,
                            message: "sessionAlreadyActive".to_string(),
                        });
                    }

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    let mfa = sqlx::query!(
                        "SELECT mfa_secret, mfa_verified FROM staff_members WHERE user_id = $1",
                        auth_data.user_id
                    )
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    if mfa.mfa_secret.is_none() {
                        return Err(Error {
                            status: StatusCode::BAD_REQUEST,
                            message: "mfaNotSetup".to_string(),
                        });
                    }

                    let secret =
                        thotp::encoding::decode(&mfa.mfa_secret.unwrap(), data_encoding::BASE32)
                            .map_err(Error::new)?;

                    let (result, _discrepancy) = thotp::verify_totp(&otp, &secret, 0).unwrap();

                    if !result {
                        return Err(Error {
                            status: StatusCode::BAD_REQUEST,
                            message: "Invalid OTP entered".to_string(),
                        });
                    }

                    sqlx::query!(
                        "UPDATE staffpanel__authchain SET state = 'active' WHERE token = $1",
                        login_token
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    sqlx::query!(
                        "UPDATE staff_members SET mfa_verified = TRUE WHERE user_id = $1",
                        auth_data.user_id
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                AuthorizeAction::Logout { login_token } => {
                    // Just delete the auth, no point in even erroring if it doesn't exist
                    let row = sqlx::query!(
                        "DELETE FROM staffpanel__authchain WHERE token = $1",
                        login_token
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::OK, row.rows_affected().to_string()).into_response())
                }
            }
        }
        PanelQuery::Hello {
            login_token,
            version,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            if version != HELLO_VERSION {
                return Ok((StatusCode::BAD_REQUEST, "Invalid version".to_string()).into_response());
            }

            // Get permissions
            let staff_member =
                super::auth::get_staff_member(&state.pool, &state.cache_http, &auth_data.user_id)
                    .await
                    .map_err(Error::new)?;

            let mut target_types: Vec<TargetType> = Vec::new();

            for target_type in TargetType::VARIANTS {
                let variant = TargetType::from_str(target_type).map_err(Error::new)?;
                target_types.push(variant);
            }

            Ok((
                StatusCode::OK,
                Json(
                    Hello {
                        instance_config: InstanceConfig {
                            description: {
                                if *crate::config::CURRENT_ENV == "staging" {
                                    "Arcadia Staging Panel Instance".to_string()
                                } else {
                                    "Arcadia Production Panel Instance".to_string()
                                }
                            },
                            warnings: vec![
                                "The panel is currently undergoing large-scale changes while it is being rewritten. Please report any bugs you find to the staff team.".to_string(),
                            ],
                        },
                        auth_data,
                        staff_member,
                        core_constants: CoreConstants {
                            frontend_url: crate::config::CONFIG.frontend_url.get().clone(),
                            infernoplex_url: crate::config::CONFIG.infernoplex_url.clone(),
                            popplio_url: crate::config::CONFIG.popplio_url.clone(),
                            htmlsanitize_url: crate::config::CONFIG.htmlsanitize_url.clone(),
                            cdn_url: crate::config::CONFIG.cdn_url.clone(),
                            servers: PanelServers {
                                main: crate::config::CONFIG.servers.main.to_string(),
                                staff: crate::config::CONFIG.servers.staff.to_string(),
                                testing: crate::config::CONFIG.servers.testing.to_string(),
                            },
                        },
                        target_types,
                    }
                )
            )
                .into_response())
        }
        PanelQuery::BaseAnalytics { login_token } => {
            super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let bot_counts = sqlx::query!("SELECT type, COUNT(*) FROM bots GROUP BY type")
                .fetch_all(&state.pool)
                .await
                .map_err(Error::new)?;

            let server_counts = sqlx::query!("SELECT type, COUNT(*) FROM servers GROUP BY type")
                .fetch_all(&state.pool)
                .await
                .map_err(Error::new)?;

            let ticket_counts = sqlx::query!("SELECT open, COUNT(*) FROM tickets GROUP BY open")
                .fetch_all(&state.pool)
                .await
                .map_err(Error::new)?;

            let total_users = sqlx::query!("SELECT COUNT(*) FROM users")
                .fetch_one(&state.pool)
                .await
                .map_err(Error::new)?;

            let total_changelogs = sqlx::query!("SELECT COUNT(*) FROM changelogs")
                .fetch_one(&state.pool)
                .await
                .map_err(Error::new)?;

            Ok((
                StatusCode::OK,
                Json(BaseAnalytics {
                    bot_counts: bot_counts
                        .iter()
                        .map(|b| (b.r#type.clone(), b.count.unwrap_or_default()))
                        .collect(),
                    server_counts: server_counts
                        .iter()
                        .map(|s| (s.r#type.clone(), s.count.unwrap_or_default()))
                        .collect(),
                    ticket_counts: ticket_counts
                        .iter()
                        .map(|t| (t.open, t.count.unwrap_or_default()))
                        .collect(),
                    total_users: total_users.count.unwrap_or_default(),
                    changelogs_count: total_changelogs.count.unwrap_or_default(),
                }),
            )
                .into_response())
        }
        PanelQuery::GetUser {
            login_token,
            user_id,
        } => {
            super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user = crate::impls::dovewing::get_platform_user(
                &state.pool,
                DovewingSource::Discord(state.cache_http.clone()),
                &user_id,
            )
            .await
            .map_err(Error::new)?;

            Ok((StatusCode::OK, Json(user)).into_response())
        }
        PanelQuery::BotQueue { login_token } => {
            super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let queue = sqlx::query!(
                "SELECT bot_id, client_id, last_claimed, claimed_by, type, approval_note, short, 
                invite, votes, shards, library, invite_clicks, clicks, servers 
                FROM bots WHERE type = 'pending' OR type = 'claimed' ORDER BY created_at"
            )
            .fetch_all(&state.pool)
            .await
            .map_err(Error::new)?;

            let mut bots = Vec::new();

            for bot in queue {
                let owners = crate::impls::utils::get_entity_managers(
                    TargetType::Bot,
                    &bot.bot_id,
                    &state.pool,
                )
                .await
                .map_err(Error::new)?;

                let user = crate::impls::dovewing::get_platform_user(
                    &state.pool,
                    DovewingSource::Discord(state.cache_http.clone()),
                    &bot.bot_id,
                )
                .await
                .map_err(Error::new)?;

                bots.push(PartialEntity::Bot(PartialBot {
                    bot_id: bot.bot_id,
                    client_id: bot.client_id,
                    user,
                    claimed_by: bot.claimed_by,
                    last_claimed: bot.last_claimed,
                    approval_note: bot.approval_note,
                    short: bot.short,
                    r#type: bot.r#type,
                    votes: bot.votes,
                    shards: bot.shards,
                    library: bot.library,
                    invite_clicks: bot.invite_clicks,
                    clicks: bot.clicks,
                    servers: bot.servers,
                    mentionable: owners.mentionables(),
                    invite: bot.invite,
                }));
            }

            Ok((StatusCode::OK, Json(bots)).into_response())
        }
        PanelQuery::ExecuteRpc {
            login_token,
            target_type,
            method,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let resp = method
                .handle(RPCHandle {
                    pool: state.pool.clone(),
                    cache_http: state.cache_http.clone(),
                    user_id: auth_data.user_id,
                    target_type,
                })
                .await;

            match resp {
                Ok(r) => match r {
                    crate::rpc::core::RPCSuccess::NoContent => {
                        Ok((StatusCode::NO_CONTENT, "").into_response())
                    }
                    crate::rpc::core::RPCSuccess::Content(c) => {
                        Ok((StatusCode::OK, c).into_response())
                    }
                },
                Err(e) => Ok((StatusCode::BAD_REQUEST, e.to_string()).into_response()),
            }
        }
        PanelQuery::GetRpcMethods {
            login_token,
            filtered,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            let mut rpc_methods = Vec::new();

            for method in crate::rpc::core::RPCMethod::VARIANTS {
                let variant = crate::rpc::core::RPCMethod::from_str(method).map_err(Error::new)?;

                if filtered {
                    let required_perm = format!("rpc.{}", variant).into();
                    if !perms::has_perm(&user_perms, &required_perm) {
                        continue;
                    }
                }

                let action = RPCWebAction {
                    id: method.to_string(),
                    label: variant.label(),
                    description: variant.description(),
                    supported_target_types: variant.supported_target_types(),
                    fields: variant.method_fields(),
                };

                rpc_methods.push(action);
            }

            Ok((StatusCode::OK, Json(rpc_methods)).into_response())
        }
        PanelQuery::GetRpcLogEntries { login_token } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            if !perms::has_perm(&user_perms, &"rpc_logs.view".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to view rpc logs [rpc_logs.view]".to_string(),
                )
                    .into_response());
            }

            let entries = sqlx::query!(
                "SELECT id, user_id, method, data, state, created_at FROM rpc_logs ORDER BY created_at DESC"
            )
            .fetch_all(&state.pool)
            .await
            .map_err(Error::new)?;

            let mut rpc_log = vec![];

            for entry in entries {
                rpc_log.push(RPCLogEntry {
                    id: entry.id.to_string(),
                    user_id: entry.user_id,
                    method: entry.method,
                    data: entry.data,
                    state: entry.state,
                    created_at: entry.created_at,
                });
            }

            Ok((StatusCode::OK, Json(rpc_log)).into_response())
        }
        PanelQuery::SearchEntitys {
            login_token,
            target_type,
            query,
        } => {
            super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            match target_type {
                TargetType::Bot => {
                    let queue = sqlx::query!(
                        "
                        SELECT bot_id, client_id, type, votes, shards, library, invite_clicks, clicks,
                        servers, last_claimed, claimed_by, approval_note, short, invite FROM bots 
                        INNER JOIN internal_user_cache__discord discord_users ON bots.bot_id = discord_users.id
                        WHERE bot_id = $1 OR client_id = $1 OR discord_users.username ILIKE $2 ORDER BY bots.created_at
                        ",
                        query,
                        format!("%{}%", query)
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut bots = Vec::new();

                    for bot in queue {
                        let owners = crate::impls::utils::get_entity_managers(
                            TargetType::Bot,
                            &bot.bot_id,
                            &state.pool,
                        )
                        .await
                        .map_err(Error::new)?;

                        let user = crate::impls::dovewing::get_platform_user(
                            &state.pool,
                            DovewingSource::Discord(state.cache_http.clone()),
                            &bot.bot_id,
                        )
                        .await
                        .map_err(Error::new)?;

                        bots.push(PartialEntity::Bot(PartialBot {
                            bot_id: bot.bot_id,
                            client_id: bot.client_id,
                            user,
                            r#type: bot.r#type,
                            votes: bot.votes,
                            shards: bot.shards,
                            library: bot.library,
                            invite_clicks: bot.invite_clicks,
                            clicks: bot.clicks,
                            servers: bot.servers,
                            claimed_by: bot.claimed_by,
                            last_claimed: bot.last_claimed,
                            approval_note: bot.approval_note,
                            short: bot.short,
                            mentionable: owners.mentionables(),
                            invite: bot.invite,
                        }));
                    }

                    Ok((StatusCode::OK, Json(bots)).into_response())
                }
                TargetType::Server => {
                    let queue = sqlx::query!(
                        "
                        SELECT server_id, name, total_members, online_members, short, type, votes, invite_clicks, 
                        clicks, nsfw, tags, premium, claimed_by, last_claimed FROM servers
                        WHERE server_id = $1 OR name ILIKE $2 ORDER BY created_at
                        ",
                        query,
                        format!("%{}%", query)
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut servers = Vec::new();

                    for server in queue {
                        let owners = crate::impls::utils::get_entity_managers(
                            TargetType::Server,
                            &server.server_id,
                            &state.pool,
                        )
                        .await
                        .map_err(Error::new)?;

                        servers.push(PartialEntity::Server(PartialServer {
                            server_id: server.server_id.clone(),
                            name: server.name,
                            avatar: format!(
                                "{}/servers/avatars/{}.webp",
                                crate::config::CONFIG.cdn_url,
                                server.server_id
                            ),
                            total_members: server.total_members,
                            online_members: server.online_members,
                            short: server.short,
                            r#type: server.r#type,
                            votes: server.votes,
                            invite_clicks: server.invite_clicks,
                            clicks: server.clicks,
                            nsfw: server.nsfw,
                            tags: server.tags,
                            premium: server.premium,
                            claimed_by: server.claimed_by,
                            last_claimed: server.last_claimed,
                            mentionable: owners.mentionables(),
                        }));
                    }

                    Ok((StatusCode::OK, Json(servers)).into_response())
                }
                _ => Ok((
                    StatusCode::NOT_IMPLEMENTED,
                    "Searching this target type is not implemented".to_string(),
                )
                    .into_response()),
            }
        }
        PanelQuery::UploadCdnFileChunk { login_token, chunk } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            if !perms::has_perm(&user_perms, &"cdn.upload_chunk".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to upload chunks to the CDN right now [cdn.upload_chunk]".to_string(),
                )
                    .into_response());
            }

            info!("Got chunk with len={}", chunk.len());

            // Check that length of chunk is not greater than 100MB
            if chunk.len() > 100_000_000 {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Chunk size is too large".to_string(),
                )
                    .into_response());
            }

            // Check that chunk is not empty
            if chunk.is_empty() {
                return Ok((StatusCode::BAD_REQUEST, "Chunk is empty".to_string()).into_response());
            }

            // Create chunk ID
            let chunk_id = botox::crypto::gen_random(32);

            // Keep looping until we get a free chunk ID
            let mut tries = 0;

            while tries < 10 {
                if !state.cdn_file_chunks_cache.contains_key(&chunk_id) {
                    state
                        .cdn_file_chunks_cache
                        .insert(chunk_id.clone(), chunk)
                        .await;

                    return Ok((StatusCode::OK, chunk_id).into_response());
                }

                tries += 1;
            }

            Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate a chunk ID".to_string(),
            )
                .into_response())
        }
        PanelQuery::ListCdnScopes { login_token } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            if !perms::has_perm(&user_perms, &"cdn.list_scopes".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to list the CDN's scopes right now [cdn.list_scopes]".to_string(),
                )
                    .into_response());
            }

            Ok((
                StatusCode::OK,
                Json(crate::config::CONFIG.panel.cdn_scopes.get().clone()),
            )
                .into_response())
        }
        PanelQuery::GetMainCdnScope { login_token } => {
            super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            Ok((
                StatusCode::OK,
                crate::config::CONFIG.panel.main_scope.clone(),
            )
                .into_response())
        }
        PanelQuery::UpdateCdnAsset {
            login_token,
            name,
            path,
            action,
            cdn_scope,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            // Get cdn path from cdn_scope hashmap
            let cdn_scopes = crate::config::CONFIG.panel.cdn_scopes.get();
            let Some(cdn_path) = cdn_scopes.get(&cdn_scope) else {
                return Ok(
                    (StatusCode::BAD_REQUEST, "Invalid CDN scope".to_string()).into_response()
                );
            };

            fn validate_name(name: &str) -> Result<(), crate::Error> {
                const ALLOWED_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.:%$[](){}$@! ";

                // 1. Ensure all chars of name are in ALLOWED_CHARS
                // 2. Ensure name does not contain a slash
                // 3. Ensure name does not contain a backslash
                // 4. Ensure name does not start with a dot
                if name.chars().any(|c| !ALLOWED_CHARS.contains(c))
                    || name.contains('/')
                    || name.contains('\\')
                    || name.starts_with('.')
                {
                    return Err(
                        "Asset name cannot contain disallowed characters, slashes or backslashes or start with a dot"
                            .into(),
                    );
                }

                Ok(())
            }

            fn validate_path(path: &str) -> Result<(), crate::Error> {
                const ALLOWED_CHARS: &str =
                    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.:%$/ ";

                // 1. Ensure all chars of name are in ALLOWED_CHARS
                // 2. Ensure path does not contain a dot-dot (path escape)
                // 3. Ensure path does not contain a double slash
                // 4. Ensure path does not contain a backslash
                // 5. Ensure path does not start with a slash
                if path.chars().any(|c| !ALLOWED_CHARS.contains(c))
                    || path.contains("..")
                    || path.contains("//")
                    || path.contains('\\')
                    || path.starts_with('/')
                {
                    return Err("Asset path cannot contain non-ASCII characters, dot-dots, doubleslashes, backslashes or start with a slash".into());
                }

                Ok(())
            }

            validate_name(&name).map_err(Error::new)?;
            validate_path(&path).map_err(Error::new)?;

            // Get asset path and final resolved path
            let asset_path = if path.is_empty() {
                cdn_path.path.to_string()
            } else {
                format!("{}/{}", cdn_path.path, path)
            };

            let asset_final_path = if name.is_empty() {
                asset_path.clone()
            } else {
                format!("{}/{}", asset_path, name)
            };

            match action {
                CdnAssetAction::ListPath => {
                    if !has_cdn_perm(&user_perms, &cdn_scope, "list") {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to list CDN assets right now [list]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    match std::fs::metadata(&asset_path) {
                        Ok(m) => {
                            if !m.is_dir() {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Asset path already exists and is not a directory".to_string(),
                                )
                                    .into_response());
                            }
                        }
                        Err(e) => {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                "Fetching asset metadata failed: ".to_string() + &e.to_string(),
                            )
                                .into_response());
                        }
                    }

                    let mut files = Vec::new();

                    for entry in std::fs::read_dir(&asset_path).map_err(Error::new)? {
                        let entry = entry.map_err(Error::new)?;

                        let meta = entry.metadata().map_err(Error::new)?;

                        let efn = entry.file_name();
                        let Some(name) = efn.to_str() else {
                            continue;
                        };

                        files.push(CdnAssetItem {
                            name: name.to_string(),
                            path: entry
                                .path()
                                .to_str()
                                .unwrap_or_default()
                                .to_string()
                                .replace(&cdn_path.path, ""),
                            size: meta.len(),
                            last_modified: meta
                                .modified()
                                .map_err(Error::new)?
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_err(Error::new)?
                                .as_secs(),
                            is_dir: meta.is_dir(),
                            permissions: meta.permissions().mode(),
                        });
                    }

                    Ok((StatusCode::OK, Json(files)).into_response())
                }
                CdnAssetAction::ReadFile => {
                    if !has_cdn_perm(&user_perms, &cdn_scope, "read_file") {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to read CDN files right now [read_file]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    match std::fs::metadata(&asset_final_path) {
                        Ok(m) => {
                            if !m.is_file() {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Asset path is not a file".to_string(),
                                )
                                    .into_response());
                            }
                        }
                        Err(e) => {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                "Fetching asset metadata failed: ".to_string() + &e.to_string(),
                            )
                                .into_response());
                        }
                    }

                    let file = match tokio::fs::File::open(&asset_final_path).await {
                        Ok(file) => file,
                        Err(e) => {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                "Reading file failed: ".to_string() + &e.to_string(),
                            )
                                .into_response());
                        }
                    };

                    let stream = tokio_util::io::ReaderStream::new(file);
                    let body = StreamBody::new(stream);

                    let headers = [(axum::http::header::CONTENT_TYPE, "application/octet-stream")];

                    Ok((headers, body).into_response())
                }
                CdnAssetAction::CreateFolder => {
                    if !has_cdn_perm(&user_perms, &cdn_scope, "create_folder") {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create CDN folders right now [create_folder]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    match std::fs::metadata(&asset_final_path) {
                        Ok(_) => {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                "Asset path already exists".to_string(),
                            )
                                .into_response());
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::NotFound {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Fetching asset metadata failed due to unknown error: "
                                        .to_string()
                                        + &e.to_string(),
                                )
                                    .into_response());
                            }
                        }
                    }

                    // Create path
                    std::fs::DirBuilder::new()
                        .recursive(true)
                        .create(&asset_final_path)
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                CdnAssetAction::AddFile {
                    overwrite,
                    chunks,
                    sha512,
                } => {
                    if !has_cdn_perm(&user_perms, &cdn_scope, "add_file") {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to add CDN files right now [cdn.add_file]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    if chunks.is_empty() {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "No chunks were provided".to_string(),
                        )
                            .into_response());
                    }

                    if chunks.len() > 100_000 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Too many chunks were provided".to_string(),
                        )
                            .into_response());
                    }

                    for chunk in &chunks {
                        if !state.cdn_file_chunks_cache.contains_key(chunk) {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                "Chunk does not exist".to_string(),
                            )
                                .into_response());
                        }
                    }

                    // Check if the asset exists
                    match std::fs::metadata(&asset_final_path) {
                        Ok(m) => {
                            if overwrite {
                                if m.is_dir() {
                                    return Ok((
                                        StatusCode::BAD_REQUEST,
                                        "Asset to be replaced is a directory".to_string(),
                                    )
                                        .into_response());
                                }
                            } else {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Asset already exists".to_string(),
                                )
                                    .into_response());
                            }
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::NotFound {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Fetching asset metadata failed due to unknown error: "
                                        .to_string()
                                        + &e.to_string(),
                                )
                                    .into_response());
                            }
                        }
                    }

                    match std::fs::metadata(&asset_path) {
                        Ok(m) => {
                            if !m.is_dir() {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Asset path already exists and is not a directory".to_string(),
                                )
                                    .into_response());
                            }
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::NotFound {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Fetching asset metadata failed due to unknown error: "
                                        .to_string()
                                        + &e.to_string(),
                                )
                                    .into_response());
                            } else {
                                // Create path
                                std::fs::DirBuilder::new()
                                    .recursive(true)
                                    .create(&asset_path)
                                    .map_err(Error::new)?;
                            }
                        }
                    }

                    {
                        let tmp_file_path = format!(
                            "/tmp/arcadia-cdn-file{}@{}",
                            botox::crypto::gen_random(32),
                            asset_final_path.replace('/', ">")
                        );

                        let mut temp_file = tokio::fs::File::create(&tmp_file_path)
                            .await
                            .map_err(Error::new)?;

                        // For each chunk, fetch and add to file
                        for chunk in chunks {
                            let chunk = state
                                .cdn_file_chunks_cache
                                .remove(&chunk)
                                .await
                                .ok_or_else(|| {
                                    Error::new("Chunk ".to_string() + &chunk + " does not exist")
                                })?;

                            temp_file.write_all(&chunk).await.map_err(Error::new)?;
                        }

                        // Sync file
                        temp_file.sync_all().await.map_err(Error::new)?;

                        // Close file
                        drop(temp_file);

                        // Calculate sha512 of file
                        let mut hasher = Sha512::new();

                        let mut file = tokio::fs::File::open(&tmp_file_path)
                            .await
                            .map_err(Error::new)?;

                        let mut file_buf = Vec::new();
                        file.read_to_end(&mut file_buf).await.map_err(Error::new)?;

                        hasher.update(&file_buf);

                        let hash = hasher.finalize();

                        let hash_expected = data_encoding::HEXLOWER.encode(&hash);

                        if sha512 != hash_expected {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                "SHA512 hash does not match".to_string(),
                            )
                                .into_response());
                        }

                        // Rename temp file to final path
                        tokio::fs::copy(&tmp_file_path, &asset_final_path)
                            .await
                            .map_err(Error::new)?;

                        // Delete temp file
                        tokio::fs::remove_file(&tmp_file_path)
                            .await
                            .map_err(Error::new)?;
                    }

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                CdnAssetAction::CopyFile {
                    overwrite,
                    delete_original,
                    copy_to,
                } => {
                    if !has_cdn_perm(&user_perms, &cdn_scope, "copy_file") {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to copy files right now [copy_file]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    validate_path(&copy_to).map_err(Error::new)?;

                    let copy_to = if copy_to.is_empty() {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "copy_to location cannot be empty".to_string(),
                        )
                            .into_response());
                    } else {
                        format!("{}/{}", cdn_path.path, copy_to)
                    };

                    match std::fs::metadata(&copy_to) {
                        Ok(m) => {
                            if !m.is_dir() && !overwrite {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "copy_to location already exists".to_string(),
                                )
                                    .into_response());
                            }
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::NotFound {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Fetching asset metadata failed due to unknown error: "
                                        .to_string()
                                        + &e.to_string(),
                                )
                                    .into_response());
                            }
                        }
                    }

                    match std::fs::metadata(&asset_final_path) {
                        Ok(m) => {
                            if m.is_symlink() || m.is_file() {
                                if delete_original {
                                    // This is just a rename operation
                                    std::fs::rename(&asset_final_path, &copy_to).map_err(|e| {
                                        Error::new(format!(
                                            "Failed to rename file: {} from {} to {}",
                                            e, &asset_final_path, &copy_to
                                        ))
                                    })?;
                                } else {
                                    // This is a copy operation
                                    std::fs::copy(&asset_final_path, &copy_to)
                                        .map_err(Error::new)?;
                                }
                            } else if m.is_dir() {
                                if delete_original {
                                    // This is a rename operation
                                    fn rename_dir_all(
                                        src: impl AsRef<std::path::Path>,
                                        dst: impl AsRef<std::path::Path>,
                                    ) -> std::io::Result<()> {
                                        std::fs::create_dir_all(&dst)?;
                                        for entry in std::fs::read_dir(src)? {
                                            let entry = entry?;
                                            let ty = entry.file_type()?;
                                            if ty.is_dir() {
                                                rename_dir_all(
                                                    entry.path(),
                                                    dst.as_ref().join(entry.file_name()),
                                                )?;
                                            } else {
                                                std::fs::rename(
                                                    entry.path(),
                                                    dst.as_ref().join(entry.file_name()),
                                                )?;
                                            }
                                        }
                                        Ok(())
                                    }

                                    rename_dir_all(&asset_final_path, &copy_to)
                                        .map_err(Error::new)?;

                                    // Delete original directory
                                    std::fs::remove_dir_all(&asset_final_path)
                                        .map_err(Error::new)?;
                                } else {
                                    // This is a recursive copy operation
                                    fn copy_dir_all(
                                        src: impl AsRef<std::path::Path>,
                                        dst: impl AsRef<std::path::Path>,
                                    ) -> std::io::Result<()> {
                                        std::fs::create_dir_all(&dst)?;
                                        for entry in std::fs::read_dir(src)? {
                                            let entry = entry?;
                                            let ty = entry.file_type()?;
                                            if ty.is_dir() {
                                                copy_dir_all(
                                                    entry.path(),
                                                    dst.as_ref().join(entry.file_name()),
                                                )?;
                                            } else {
                                                std::fs::copy(
                                                    entry.path(),
                                                    dst.as_ref().join(entry.file_name()),
                                                )?;
                                            }
                                        }
                                        Ok(())
                                    }

                                    copy_dir_all(&asset_final_path, &copy_to)
                                        .map_err(Error::new)?;
                                }
                            }
                        }
                        Err(e) => {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                "Could not find asset: ".to_string()
                                    + &e.to_string()
                                    + &format!(" (path: {})", &asset_final_path),
                            )
                                .into_response());
                        }
                    }

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                CdnAssetAction::Delete => {
                    if !has_cdn_perm(&user_perms, &cdn_scope, "delete") {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete CDN assets right now [delete]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if the asset exists
                    match std::fs::metadata(&asset_final_path) {
                        Ok(m) => {
                            if m.is_symlink() || m.is_file() {
                                // Delete the symlink
                                std::fs::remove_file(asset_final_path).map_err(Error::new)?;
                            } else if m.is_dir() {
                                // Delete the directory
                                std::fs::remove_dir_all(asset_final_path).map_err(Error::new)?;
                            }

                            Ok((StatusCode::NO_CONTENT, "").into_response())
                        }
                        Err(e) => Ok((
                            StatusCode::BAD_REQUEST,
                            "Could not find asset: ".to_string() + &e.to_string(),
                        )
                            .into_response()),
                    }
                }
                CdnAssetAction::PersistGit {
                    message,
                    current_dir,
                } => {
                    if !has_cdn_perm(&user_perms, &cdn_scope, "persist_git") {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to persist CDN git right now [cdn.persist_git]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    let mut cmd_output = indexmap::IndexMap::new();

                    // Use git rev-parse --show-toplevel to get the root of the repo
                    let output = tokio::process::Command::new("git")
                        .arg("rev-parse")
                        .arg("--show-toplevel")
                        .current_dir(&asset_final_path)
                        .output()
                        .await
                        .map_err(|e| {
                            Error::new(format!("Failed to execute git rev-parse: {}", e))
                        })?;

                    let repo_root = std::str::from_utf8(&output.stdout)
                        .map_err(|e| Error::new(format!("Failed to parse git output: {}", e)))?
                        .trim()
                        .replace('\n', "")
                        .to_string();

                    cmd_output.insert("git rev-parse --show-toplevel", repo_root.clone());

                    if !output.status.success() {
                        cmd_output.insert("error", output.status.to_string());
                        return Ok((StatusCode::OK, Json(cmd_output)).into_response());
                    }

                    // If current_dir is set, then set curr dir to that
                    //
                    // Otherwise, set curr dir to the root of the repo
                    let curr_dir = if !current_dir {
                        repo_root.clone()
                    } else {
                        asset_final_path.clone()
                    };

                    cmd_output.insert("[dir]", curr_dir.clone());

                    // Run `git add .` in the current directory
                    let output = tokio::process::Command::new("git")
                        .arg("add")
                        .arg(".")
                        .current_dir(&curr_dir)
                        .env("GIT_TERMINAL_PROMPT", "0")
                        .output()
                        .await
                        .map_err(|e| Error::new(format!("Failed to execute git add: {}", e)))?;

                    cmd_output.insert(
                        "git add",
                        std::str::from_utf8(&output.stdout)
                            .map_err(|e| Error::new(format!("Failed to parse git output: {}", e)))?
                            .trim()
                            .to_string(),
                    );

                    if !output.status.success() {
                        cmd_output.insert("error", output.status.to_string());
                        return Ok((StatusCode::OK, Json(cmd_output)).into_response());
                    }

                    // Check if theres already a pending commit

                    // Run `git commit -m <message>` in the current directory
                    let output = tokio::process::Command::new("git")
                        .arg("commit")
                        .arg("-m")
                        .arg(message)
                        .env("GIT_TERMINAL_PROMPT", "0")
                        .current_dir(&curr_dir)
                        .output()
                        .await
                        .map_err(|e| Error::new(format!("Failed to execute git commit: {}", e)))?;

                    cmd_output.insert(
                        "git commit",
                        std::str::from_utf8(&output.stdout)
                            .map_err(|e| Error::new(format!("Failed to parse git output: {}", e)))?
                            .trim()
                            .to_string(),
                    );

                    if !output.status.success() {
                        cmd_output.insert("error_gc", output.status.to_string());
                    }

                    // Run `git push --force` in the current directory
                    let output = tokio::process::Command::new("git")
                        .arg("push")
                        .arg("--force")
                        .env("GIT_TERMINAL_PROMPT", "0")
                        .current_dir(&curr_dir)
                        .output()
                        .await
                        .map_err(|e| Error::new(format!("Failed to execute git push: {}", e)))?;

                    cmd_output.insert(
                        "git push",
                        std::str::from_utf8(&output.stdout)
                            .map_err(|e| Error::new(format!("Failed to parse git output: {}", e)))?
                            .trim()
                            .to_string(),
                    );

                    if !output.status.success() {
                        cmd_output.insert("error_gp", output.status.to_string());
                        return Ok((StatusCode::OK, Json(cmd_output)).into_response());
                    }

                    Ok((StatusCode::OK, Json(cmd_output)).into_response())
                }
            }
        }
        PanelQuery::UpdatePartners {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            async fn parse_partner(
                pool: &PgPool,
                partner: &CreatePartner,
            ) -> Result<(), crate::Error> {
                // Check if partner type exists
                let partner_type_exists =
                    sqlx::query!("SELECT id FROM partner_types WHERE id = $1", partner.r#type)
                        .fetch_optional(pool)
                        .await?
                        .is_some();

                if !partner_type_exists {
                    return Err("Partner type does not exist".into());
                }

                // Ensure that image has been uploaded to CDN
                // Get cdn path from cdn_scope hashmap
                let cdn_scopes = crate::config::CONFIG.panel.cdn_scopes.get();

                let Some(cdn_path) = cdn_scopes.get(&crate::config::CONFIG.panel.main_scope) else {
                    return Err("Main scope not found".into());
                };

                let path = format!("{}/avatars/partners/{}.webp", cdn_path.path, partner.id);

                match std::fs::metadata(&path) {
                    Ok(m) => {
                        if !m.is_file() {
                            return Err("Image does not exist".into());
                        }

                        if m.len() > 100_000_000 {
                            return Err("Image is too large".into());
                        }

                        if m.len() == 0 {
                            return Err("Image is empty".into());
                        }
                    }
                    Err(e) => {
                        return Err(("Fetching image metadata failed: ".to_string()
                            + &e.to_string())
                            .into());
                    }
                };

                if partner.links.is_empty() {
                    return Err("Links cannot be empty".into());
                }

                for link in &partner.links {
                    if link.name.is_empty() {
                        return Err("Link name cannot be empty".into());
                    }

                    if link.value.is_empty() {
                        return Err("Link URL cannot be empty".into());
                    }

                    if !link.value.starts_with("https://") {
                        return Err("Link URL must start with https://".into());
                    }
                }

                // Check user id
                let user_exists = sqlx::query!(
                    "SELECT user_id FROM users WHERE user_id = $1",
                    partner.user_id
                )
                .fetch_optional(pool)
                .await?
                .is_some();

                if !user_exists {
                    return Err("User does not exist".into());
                }

                Ok(())
            }

            match action {
                PartnerAction::List => {
                    let prec = sqlx::query!(
                        "SELECT id, name, short, links, type, created_at, user_id FROM partners"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut partners = Vec::new();

                    for partner in prec {
                        partners.push(Partner {
                            id: partner.id,
                            name: partner.name,
                            short: partner.short,
                            links: serde_json::from_value(partner.links).map_err(Error::new)?,
                            r#type: partner.r#type,
                            created_at: partner.created_at,
                            user_id: partner.user_id,
                        })
                    }

                    let ptrec =
                        sqlx::query!("SELECT id, name, short, icon, created_at FROM partner_types")
                            .fetch_all(&state.pool)
                            .await
                            .map_err(Error::new)?;

                    let mut partner_types = Vec::new();

                    for partner_type in ptrec {
                        partner_types.push(PartnerType {
                            id: partner_type.id,
                            name: partner_type.name,
                            short: partner_type.short,
                            icon: partner_type.icon,
                            created_at: partner_type.created_at,
                        })
                    }

                    Ok((
                        StatusCode::OK,
                        Json(Partners {
                            partners,
                            partner_types,
                        }),
                    )
                        .into_response())
                }
                PartnerAction::Create { partner } => {
                    if !perms::has_perm(&user_perms, &"partners.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create partners [partners.create]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if partner already exists
                    let partner_exists =
                        sqlx::query!("SELECT id FROM partners WHERE id = $1", partner.id)
                            .fetch_optional(&state.pool)
                            .await
                            .map_err(Error::new)?
                            .is_some();

                    if partner_exists {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Partner already exists".to_string(),
                        )
                            .into_response());
                    }

                    if let Err(e) = parse_partner(&state.pool, &partner).await {
                        return Ok((StatusCode::BAD_REQUEST, e.to_string()).into_response());
                    }

                    // Insert partner
                    sqlx::query!(
                        "INSERT INTO partners (id, name, short, links, type, user_id) VALUES ($1, $2, $3, $4, $5, $6)",
                        partner.id,
                        partner.name,
                        partner.short,
                        serde_json::to_value(partner.links).map_err(Error::new)?,
                        partner.r#type,
                        partner.user_id
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                PartnerAction::Update { partner } => {
                    if !perms::has_perm(&user_perms, &"partners.update".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update partners [partners.update]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if partner already exists
                    let partner_exists =
                        sqlx::query!("SELECT id FROM partners WHERE id = $1", partner.id)
                            .fetch_optional(&state.pool)
                            .await
                            .map_err(Error::new)?
                            .is_some();

                    if !partner_exists {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Partner does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    if let Err(e) = parse_partner(&state.pool, &partner).await {
                        return Ok((StatusCode::BAD_REQUEST, e.to_string()).into_response());
                    }

                    // Update partner
                    sqlx::query!(
                        "UPDATE partners SET name = $2, short = $3, links = $4, type = $5, user_id = $6 WHERE id = $1",
                        partner.id,
                        partner.name,
                        partner.short,
                        serde_json::to_value(partner.links).map_err(Error::new)?,
                        partner.r#type,
                        partner.user_id
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                PartnerAction::Delete { id } => {
                    if !perms::has_perm(&user_perms, &"partners.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete partners [partners.delete]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if partner exists
                    let partner_exists = sqlx::query!("SELECT id FROM partners WHERE id = $1", id)
                        .fetch_optional(&state.pool)
                        .await
                        .map_err(Error::new)?
                        .is_some();

                    if !partner_exists {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Partner does not exist".to_string(),
                        )
                            .into_response());
                    }

                    // Ensure that image has been uploaded to CDN
                    // Get cdn path from cdn_scope hashmap
                    let cdn_scopes = crate::config::CONFIG.panel.cdn_scopes.get();

                    let Some(cdn_path) = cdn_scopes.get(&crate::config::CONFIG.panel.main_scope)
                    else {
                        return Ok(
                            (StatusCode::BAD_REQUEST, "Main scope not found".to_string())
                                .into_response(),
                        );
                    };

                    let path = format!("{}/partners/{}.webp", cdn_path.path, id);

                    match std::fs::metadata(&path) {
                        Ok(m) => {
                            if m.is_symlink() || m.is_file() {
                                // Delete the symlink
                                std::fs::remove_file(path).map_err(Error::new)?;
                            } else if m.is_dir() {
                                // Delete the directory
                                std::fs::remove_dir_all(path).map_err(Error::new)?;
                            }
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::NotFound {
                                return Ok((
                                    StatusCode::BAD_REQUEST,
                                    "Fetching asset metadata failed due to unknown error: "
                                        .to_string()
                                        + &e.to_string(),
                                )
                                    .into_response());
                            }
                        }
                    };

                    sqlx::query!("DELETE FROM partners WHERE id = $1", id)
                        .execute(&state.pool)
                        .await
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateChangelog {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            match action {
                ChangelogAction::ListEntries => {
                    let rows = sqlx::query!(
                        "SELECT version, added, updated, removed, github_html, created_at, extra_description, prerelease, published FROM changelogs ORDER BY version::semver DESC"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut entries = Vec::new();

                    for row in rows {
                        entries.push(ChangelogEntry {
                            version: row.version,
                            added: row.added,
                            updated: row.updated,
                            removed: row.removed,
                            github_html: row.github_html,
                            created_at: row.created_at,
                            extra_description: row.extra_description,
                            prerelease: row.prerelease,
                            published: row.published,
                        });
                    }

                    Ok((StatusCode::OK, Json(entries)).into_response())
                }
                ChangelogAction::CreateEntry {
                    version,
                    extra_description,
                    prerelease,
                    added,
                    updated,
                    removed,
                } => {
                    if !perms::has_perm(&user_perms, &"changelogs.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create changelog entries [changelogs.create]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!(
                        "SELECT COUNT(*) FROM changelogs WHERE version = $1",
                        version
                    )
                    .fetch_one(&state.pool)
                    .await
                    .map_err(Error::new)?
                    .count
                    .unwrap_or(0)
                        > 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same version already exists".to_string(),
                        )
                            .into_response());
                    }

                    // Insert entry
                    sqlx::query!(
                        "INSERT INTO changelogs (version, extra_description, prerelease, added, updated, removed) VALUES ($1, $2, $3, $4, $5, $6)",
                        version,
                        extra_description,
                        prerelease,
                        &added,
                        &updated,
                        &removed,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                ChangelogAction::UpdateEntry {
                    version,
                    extra_description,
                    github_html,
                    prerelease,
                    added,
                    updated,
                    removed,
                    published,
                } => {
                    if !perms::has_perm(&user_perms, &"changelogs.update".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update changelog entries [changelogs.update]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!(
                        "SELECT COUNT(*) FROM changelogs WHERE version = $1",
                        version
                    )
                    .fetch_one(&state.pool)
                    .await
                    .map_err(Error::new)?
                    .count
                    .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same version does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Update entry
                    sqlx::query!(
                        "UPDATE changelogs SET extra_description = $2, github_html = $3, prerelease = $4, added = $5, updated = $6, removed = $7, published = $8 WHERE version = $1",
                        version,
                        extra_description,
                        github_html,
                        prerelease,
                        &added,
                        &updated,
                        &removed,
                        published
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                ChangelogAction::DeleteEntry { version } => {
                    if !perms::has_perm(&user_perms, &"changelogs.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete changelog entries [changelogs.delete]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!(
                        "SELECT COUNT(*) FROM changelogs WHERE version = $1",
                        version
                    )
                    .fetch_one(&state.pool)
                    .await
                    .map_err(Error::new)?
                    .count
                    .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same version does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Delete entry
                    sqlx::query!("DELETE FROM changelogs WHERE version = $1", version)
                        .execute(&state.pool)
                        .await
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateBlog {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            // TODO: Make this not require a wasteful query
            let ad = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            match action {
                BlogAction::ListEntries => {
                    let rows = sqlx::query!(
                        "SELECT itag, slug, title, description, user_id, content, created_at, draft, tags FROM blogs ORDER BY created_at DESC"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut entries = Vec::new();

                    for row in rows {
                        entries.push(BlogPost {
                            itag: row.itag.hyphenated().to_string(),
                            slug: row.slug,
                            title: row.title,
                            description: row.description,
                            user_id: row.user_id,
                            tags: row.tags,
                            content: row.content,
                            created_at: row.created_at,
                            draft: row.draft,
                        });
                    }

                    Ok((StatusCode::OK, Json(entries)).into_response())
                }
                BlogAction::CreateEntry {
                    slug,
                    title,
                    description,
                    content,
                    tags,
                } => {
                    if !perms::has_perm(&user_perms, &"blog.create_entry".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create blog entries [blog.create_entry]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Insert entry
                    sqlx::query!(
                        "INSERT INTO blogs (slug, title, description, content, tags, user_id) VALUES ($1, $2, $3, $4, $5, $6)",
                        slug,
                        title,
                        description,
                        content,
                        &tags,
                        &ad.user_id,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                BlogAction::UpdateEntry {
                    itag,
                    slug,
                    title,
                    description,
                    content,
                    tags,
                    draft,
                } => {
                    if !perms::has_perm(&user_perms, &"blog.update_entry".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update blog entries [blog.update_entry]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    let uuid = sqlx::types::uuid::Uuid::parse_str(&itag).map_err(Error::new)?;

                    // Check if entry already exists with same vesion
                    if sqlx::query!("SELECT COUNT(*) FROM blogs WHERE itag = $1", uuid)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(Error::new)?
                        .count
                        .unwrap_or(0)
                        == 0
                    {
                        return Ok(
                            (StatusCode::BAD_REQUEST, "Entry does not exist".to_string())
                                .into_response(),
                        );
                    }

                    // Update entry
                    sqlx::query!(
                        "UPDATE blogs SET slug = $2, title = $3, description = $4, content = $5, tags = $6, draft = $7 WHERE itag = $1",
                        uuid,
                        slug,
                        title,
                        description,
                        content,
                        &tags,
                        draft
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                BlogAction::DeleteEntry { itag } => {
                    if !perms::has_perm(&user_perms, &"blog.delete_entry".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete blog entries [blog.delete_entry]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    let uuid = sqlx::types::uuid::Uuid::parse_str(&itag).map_err(Error::new)?;
                    if sqlx::query!("SELECT COUNT(*) FROM blogs WHERE itag = $1", uuid)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(Error::new)?
                        .count
                        .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Delete entry
                    sqlx::query!("DELETE FROM blogs WHERE itag = $1", uuid)
                        .execute(&state.pool)
                        .await
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateStaffPositions {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            match action {
                StaffPositionAction::ListPositions => {
                    let pos = sqlx::query!("SELECT id, name, role_id, perms, corresponding_roles, icon, index, created_at FROM staff_positions ORDER BY index ASC")
                    .fetch_all(&state.pool)
                    .await
                    .map_err(|e| format!("Error while getting staff positions {}", e))
                    .map_err(Error::new)?;

                    let mut positions = Vec::new();

                    for position_data in pos {
                        positions.push(StaffPosition {
                            id: position_data.id.hyphenated().to_string(),
                            name: position_data.name,
                            role_id: position_data.role_id,
                            perms: position_data.perms,
                            corresponding_roles: serde_json::from_value(
                                position_data.corresponding_roles,
                            )
                            .map_err(Error::new)?,
                            icon: position_data.icon,
                            index: position_data.index,
                            created_at: position_data.created_at,
                        });
                    }

                    Ok((StatusCode::OK, Json(positions)).into_response())
                }
                StaffPositionAction::SwapIndex { a, b } => {
                    // Get permissions
                    let sm = super::auth::get_staff_member(
                        &state.pool,
                        &state.cache_http,
                        &auth_data.user_id,
                    )
                    .await
                    .map_err(Error::new)?;

                    if !perms::has_perm(&sm.resolved_perms, &"staff_positions.swap_index".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to swap indexes of staff positions [staff_positions.swap_index]".to_string(),
                        )
                            .into_response());
                    }

                    // Get the lowest index permission of the member
                    let mut sm_lowest_index = i32::MAX;

                    for perm in &sm.positions {
                        if perm.index < sm_lowest_index {
                            sm_lowest_index = perm.index;
                        }
                    }

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    let index_a =
                        sqlx::query!("SELECT index FROM staff_positions WHERE id::text = $1", a)
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(|e| format!("Error while getting lower position {}", e))
                            .map_err(Error::new)?
                            .index;

                    // Get the higher staff positions index
                    let index_b =
                        sqlx::query!("SELECT index FROM staff_positions WHERE id::text = $1", b)
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(|e| format!("Error while getting higher position {}", e))
                            .map_err(Error::new)?
                            .index;

                    if index_a == index_b {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Positions have the same index".to_string(),
                        )
                            .into_response());
                    }

                    // If either a or b is lower than the lowest index of the member, then error
                    if index_a <= sm_lowest_index || index_b <= sm_lowest_index {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "Either 'a' or 'b' is lower than the lowest index of the member"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Swap the indexes
                    sqlx::query!(
                        "UPDATE staff_positions SET index = $1 WHERE id::text = $2",
                        index_b,
                        a
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while updating lower position {}", e))
                    .map_err(Error::new)?;

                    sqlx::query!(
                        "UPDATE staff_positions SET index = $1 WHERE id::text = $2",
                        index_a,
                        b
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while updating higher position {}", e))
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                StaffPositionAction::SetIndex { id, index } => {
                    let uuid = sqlx::types::uuid::Uuid::parse_str(&id).map_err(Error::new)?;

                    // Get permissions
                    let sm = super::auth::get_staff_member(
                        &state.pool,
                        &state.cache_http,
                        &auth_data.user_id,
                    )
                    .await
                    .map_err(Error::new)?;

                    if !perms::has_perm(&sm.resolved_perms, &"staff_positions.set_index".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to set the indexes of staff positions [staff_positions.set_index]".to_string(),
                        )
                            .into_response());
                    }

                    if index < 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Index cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    // Get the lowest index permission of the member
                    let mut sm_lowest_index = i32::MAX;

                    for perm in &sm.positions {
                        if perm.index < sm_lowest_index {
                            sm_lowest_index = perm.index;
                        }
                    }

                    if index <= sm_lowest_index {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "Index to set is lower than or equal to the lowest index of the staff member".to_string(),
                        )
                            .into_response());
                    }

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    let curr_index =
                        sqlx::query!("SELECT index FROM staff_positions WHERE id = $1", uuid)
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(|e| format!("Error while getting position {}", e))
                            .map_err(Error::new)?
                            .index;

                    // If the current index is lower than the lowest index of the member, then error
                    if curr_index <= sm_lowest_index {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "Current index of position is lower than or equal to the lowest index of the staff member".to_string(),
                        )
                            .into_response());
                    }

                    // Shift indexes one lower
                    sqlx::query!(
                        "UPDATE staff_positions SET index = index + 1 WHERE index >= $1",
                        index
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while shifting indexes {}", e))
                    .map_err(Error::new)?;

                    // Set the index
                    sqlx::query!(
                        "UPDATE staff_positions SET index = $1 WHERE id = $2",
                        index,
                        uuid
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while updating position {}", e))
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                StaffPositionAction::CreatePosition {
                    name,
                    role_id,
                    perms,
                    index,
                    corresponding_roles,
                    icon,
                } => {
                    // Get permissions
                    let sm = super::auth::get_staff_member(
                        &state.pool,
                        &state.cache_http,
                        &auth_data.user_id,
                    )
                    .await
                    .map_err(Error::new)?;

                    if !perms::has_perm(&sm.resolved_perms, &"staff_positions.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create staff positions [staff_positions.create]".to_string(),
                        )
                            .into_response());
                    }

                    if index < 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Index cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    // Get the lowest index permission of the member
                    let mut sm_lowest_index = i32::MAX;

                    for perm in &sm.positions {
                        if perm.index < sm_lowest_index {
                            sm_lowest_index = perm.index;
                        }
                    }

                    if index <= sm_lowest_index {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "Index is lower than or equal to the lowest index of the staff member"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Shift indexes one lower
                    let mut tx = state.pool.begin().await.map_err(Error::new)?;
                    sqlx::query!(
                        "UPDATE staff_positions SET index = index + 1 WHERE index >= $1",
                        index
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while shifting indexes {}", e))
                    .map_err(Error::new)?;

                    // Ensure role id exists on the staff server
                    let role_id_snow = role_id.parse::<RoleId>().map_err(Error::new)?;
                    let role_exists = {
                        let guild = state
                            .cache_http
                            .cache
                            .guild(crate::config::CONFIG.servers.staff);

                        if let Some(guild) = guild {
                            guild.roles.get(&role_id_snow).is_some()
                        } else {
                            false
                        }
                    };

                    if !role_exists {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Role does not exist on the staff server".to_string(),
                        )
                            .into_response());
                    }

                    // Ensure all corresponding_roles exist on the named server if
                    for role in corresponding_roles.iter() {
                        let Ok(corr_server) = CorrespondingServer::from_str(&role.name) else {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                format!("Server {} is not a supported corresponding role. Supported: {:#?}", role.name, CorrespondingServer::VARIANTS),
                            )
                                .into_response());
                        };
                        let role_id_snow = role.value.parse::<RoleId>().map_err(Error::new)?;

                        let role_exists = {
                            let guild = state.cache_http.cache.guild(corr_server.get_id());

                            if let Some(guild) = guild {
                                guild.roles.get(&role_id_snow).is_some()
                            } else {
                                false
                            }
                        };

                        if !role_exists {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                format!(
                                    "Role {} does not exist on the server {}",
                                    role_id_snow,
                                    corr_server.get_id()
                                ),
                            )
                                .into_response());
                        }
                    }

                    // Create the position
                    sqlx::query!(
                        "INSERT INTO staff_positions (name, perms, corresponding_roles, icon, role_id, index) VALUES ($1, $2, $3, $4, $5, $6)",
                        name,
                        &perms,
                        serde_json::to_value(corresponding_roles).map_err(Error::new)?,
                        icon,
                        role_id,
                        index,
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while updating position {}", e))
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                StaffPositionAction::EditPosition {
                    id,
                    name,
                    role_id,
                    perms,
                    corresponding_roles,
                    icon,
                } => {
                    let uuid = sqlx::types::uuid::Uuid::parse_str(&id).map_err(Error::new)?;

                    // Get permissions
                    let sm = super::auth::get_staff_member(
                        &state.pool,
                        &state.cache_http,
                        &auth_data.user_id,
                    )
                    .await
                    .map_err(Error::new)?;

                    if !perms::has_perm(&sm.resolved_perms, &"staff_positions.edit".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to edit staff positions [staff_positions.edit]".to_string(),
                        )
                            .into_response());
                    }

                    // Get the lowest index permission of the member
                    let mut sm_lowest_index = i32::MAX;

                    for perm in &sm.positions {
                        if perm.index < sm_lowest_index {
                            sm_lowest_index = perm.index;
                        }
                    }

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    // Get the index and current permissions of the position
                    let index = sqlx::query!("SELECT perms, index, role_id FROM staff_positions WHERE id = $1 FOR UPDATE", uuid)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while getting position {}", e))
                    .map_err(Error::new)?;

                    // If the index is lower than the lowest index of the member, then error
                    if index.index <= sm_lowest_index {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "Index is lower than the lowest index of the member".to_string(),
                        )
                            .into_response());
                    }

                    // Check perms
                    if let Err(e) = perms::check_patch_changes(
                        &sm.resolved_perms,
                        &index
                            .perms
                            .iter()
                            .map(|x| Permission::from_string(x))
                            .collect::<Vec<Permission>>(),
                        &perms
                            .iter()
                            .map(|x| Permission::from_string(x))
                            .collect::<Vec<Permission>>(),
                    ) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            format!(
                                "You do not have permission to edit the following perms: {}",
                                e
                            ),
                        )
                            .into_response());
                    }

                    // Ensure role id exists on the staff server
                    let role_id_snow = role_id.parse::<RoleId>().map_err(Error::new)?;
                    let role_exists = {
                        let guild = state
                            .cache_http
                            .cache
                            .guild(crate::config::CONFIG.servers.staff);

                        if let Some(guild) = guild {
                            guild.roles.get(&role_id_snow).is_some()
                        } else {
                            false
                        }
                    };

                    if !role_exists {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Role does not exist on the staff server".to_string(),
                        )
                            .into_response());
                    }

                    // Ensure all corresponding_roles exist on the named server if
                    for role in corresponding_roles.iter() {
                        let Ok(corr_server) = CorrespondingServer::from_str(&role.name) else {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                format!("Server {} is not a supported corresponding role. Supported: {:#?}", role.name, CorrespondingServer::VARIANTS),
                            )
                                .into_response());
                        };
                        let role_id_snow = role.value.parse::<RoleId>().map_err(Error::new)?;

                        let role_exists = {
                            let guild = state.cache_http.cache.guild(corr_server.get_id());

                            if let Some(guild) = guild {
                                guild.roles.get(&role_id_snow).is_some()
                            } else {
                                false
                            }
                        };

                        if !role_exists {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                format!(
                                    "Role {} does not exist on the server {}",
                                    role_id_snow,
                                    corr_server.get_id()
                                ),
                            )
                                .into_response());
                        }
                    }

                    // Update the position
                    sqlx::query!(
                        "UPDATE staff_positions SET name = $1, perms = $2, corresponding_roles = $3, role_id = $4, icon = $5 WHERE id = $6", 
                        name,
                        &perms,
                        serde_json::to_value(corresponding_roles).map_err(Error::new)?,
                        role_id,
                        icon,
                        uuid
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while updating position {}", e))
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                StaffPositionAction::DeletePosition { id } => {
                    let uuid = sqlx::types::uuid::Uuid::parse_str(&id).map_err(Error::new)?;

                    // Get permissions
                    let sm = super::auth::get_staff_member(
                        &state.pool,
                        &state.cache_http,
                        &auth_data.user_id,
                    )
                    .await
                    .map_err(Error::new)?;

                    if !perms::has_perm(&sm.resolved_perms, &"staff_positions.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete staff positions [staff_positions.delete]".to_string(),
                        )
                            .into_response());
                    }

                    // Get the lowest index permission of the member
                    let mut sm_lowest_index = i32::MAX;

                    for perm in &sm.positions {
                        if perm.index < sm_lowest_index {
                            sm_lowest_index = perm.index;
                        }
                    }

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    // Get the index and current permissions of the position
                    let index = sqlx::query!("SELECT perms, index, role_id FROM staff_positions WHERE id = $1 FOR UPDATE", uuid)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while getting position {}", e))
                    .map_err(Error::new)?;

                    // If the index is lower than the lowest index of the member, then error
                    if index.index <= sm_lowest_index {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "Index is lower than the lowest index of the member".to_string(),
                        )
                            .into_response());
                    }

                    // Check perms
                    if let Err(e) = perms::check_patch_changes(
                        &sm.resolved_perms,
                        &index
                            .perms
                            .iter()
                            .map(|x| Permission::from_string(x))
                            .collect::<Vec<Permission>>(),
                        &Vec::new(),
                    ) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            format!("You do not have permission to edit the following perms [neeed to delete position]: {}", e),
                        )
                            .into_response());
                    }

                    // Delete the position
                    sqlx::query!("DELETE FROM staff_positions WHERE id = $1", uuid)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| format!("Error while deleting position {}", e))
                        .map_err(Error::new)?;

                    // Shift back indexes one lower
                    sqlx::query!(
                        "UPDATE staff_positions SET index = index - 1 WHERE index > $1",
                        index.index
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while shifting indexes {}", e))
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateStaffMembers {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            match action {
                StaffMemberAction::ListMembers => {
                    let ids = sqlx::query!("SELECT user_id FROM staff_members")
                        .fetch_all(&state.pool)
                        .await
                        .map_err(|e| format!("Error while getting staff members {}", e))
                        .map_err(Error::new)?;

                    let mut members = Vec::new();

                    for id in ids {
                        let member = super::auth::get_staff_member(
                            &state.pool,
                            &state.cache_http,
                            &id.user_id,
                        )
                        .await
                        .map_err(Error::new)?;

                        members.push(member);
                    }

                    Ok((StatusCode::OK, Json(members)).into_response())
                }
                StaffMemberAction::EditMember {
                    user_id,
                    perm_overrides,
                    no_autosync,
                    unaccounted,
                } => {
                    // Get permissions
                    let sm = super::auth::get_staff_member(
                        &state.pool,
                        &state.cache_http,
                        &auth_data.user_id,
                    )
                    .await
                    .map_err(Error::new)?;

                    // Get permissions of target
                    let sm_target =
                        super::auth::get_staff_member(&state.pool, &state.cache_http, &user_id)
                            .await
                            .map_err(Error::new)?;

                    if !perms::has_perm(&sm.resolved_perms, &"staff_members.edit".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to edit staff members [staff_members.edit]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Get the lowest index permission of the member
                    let mut sm_lowest_index = i32::MAX;

                    for perm in &sm.positions {
                        if perm.index < sm_lowest_index {
                            sm_lowest_index = perm.index;
                        }
                    }

                    // Get the lowest index permission of the target
                    let mut sm_target_lowest_index = i32::MAX;

                    for perm in &sm_target.positions {
                        if perm.index < sm_target_lowest_index {
                            sm_target_lowest_index = perm.index;
                        }
                    }

                    // If the target has a lower index than the member, then error
                    if sm_target_lowest_index < sm_lowest_index {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "Target has a lower index than the member".to_string(),
                        )
                            .into_response());
                    }

                    // Check perms currently with override versus perms without override
                    let mut resolved_perms_without_overrides = sm_target.resolved_perms.clone();

                    for perm in &perm_overrides {
                        resolved_perms_without_overrides.retain(|p| p != perm);
                    }

                    if let Err(e) = perms::check_patch_changes(
                        &sm.resolved_perms,
                        &sm_target.resolved_perms,
                        &resolved_perms_without_overrides,
                    ) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            format!(
                                "You do not have permission to edit the following perms: {}",
                                e
                            ),
                        )
                            .into_response());
                    }

                    // Then update
                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    // Lock the member for update
                    sqlx::query!("SELECT perm_overrides, no_autosync, unaccounted FROM staff_members WHERE user_id = $1 FOR UPDATE", user_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while getting member {}", e))
                    .map_err(Error::new)?;

                    // Update the member
                    sqlx::query!("UPDATE staff_members SET perm_overrides = $1, no_autosync = $2, unaccounted = $3 WHERE user_id = $4",
                        &perm_overrides,
                        no_autosync,
                        unaccounted,
                        user_id
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Error while updating member {}", e))
                    .map_err(Error::new)?;

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateStaffDisciplinaryType {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            match action {
                StaffDisciplinaryTypeAction::ListDisciplinaryTypes => {
                    let rows = sqlx::query!(
                        "SELECT id, name, description, self_assignable, perm_limits, additory, needs_approval, EXTRACT(epoch FROM max_expiry) AS max_expiry, created_at FROM staff_disciplinary_types ORDER BY created_at DESC"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut entries = Vec::new();

                    for row in rows {
                        entries.push(StaffDisciplinaryType {
                            id: row.id,
                            name: row.name,
                            description: row.description,
                            self_assignable: row.self_assignable,
                            perm_limits: row.perm_limits,
                            additory: row.additory,
                            needs_approval: row.needs_approval,
                            max_expiry: row.max_expiry.map(|d| {
                                // Convert to i64
                                d.to_f64().unwrap_or_default()
                            }),
                            created_at: row.created_at,
                        });
                    }

                    Ok((StatusCode::OK, Json(entries)).into_response())
                }
                StaffDisciplinaryTypeAction::CreateDisciplinaryType {
                    id,
                    name,
                    description,
                    self_assignable,
                    perm_limits,
                    additory,
                    needs_approval,
                    max_expiry,
                } => {
                    if !perms::has_perm(&user_perms, &"staff_disciplinary_types.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create staff disciplinary types [staff_disciplinary_types.create]".to_string(),
                        )
                            .into_response());
                    }

                    if let Err(e) = perms::check_patch_changes(
                        &user_perms,
                        &Vec::new(),
                        &perm_limits
                            .iter()
                            .map(|x| Permission::from_string(x))
                            .collect::<Vec<Permission>>(),
                    ) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            format!(
                                "You do not have permission to edit the following perms: {}",
                                e
                            ),
                        )
                            .into_response());
                    }

                    // Insert entry
                    sqlx::query!(
                        "INSERT INTO staff_disciplinary_types (id, name, description, self_assignable, perm_limits, additory, needs_approval, max_expiry) VALUES ($1, $2, $3, $4, $5, $6, $7, make_interval(secs => $8))",
                        id,
                        name,
			            description,
                        self_assignable,
                        &perm_limits,
                        additory,
			            needs_approval,
			            max_expiry,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                StaffDisciplinaryTypeAction::EditDisciplinaryType {
                    id,
                    name,
                    description,
                    self_assignable,
                    perm_limits,
                    additory,
                    needs_approval,
                    max_expiry,
                } => {
                    if !perms::has_perm(&user_perms, &"staff_disciplinary_types.update".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update staff disciplinary types [staff_disciplinary_types.update]".to_string(),
                        )
                            .into_response());
                    }

                    if let Err(e) = perms::check_patch_changes(
                        &user_perms,
                        &Vec::new(),
                        &perm_limits
                            .iter()
                            .map(|x| Permission::from_string(x))
                            .collect::<Vec<Permission>>(),
                    ) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            format!(
                                "You do not have permission to edit the following perms: {}",
                                e
                            ),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!(
                        "SELECT COUNT(*) FROM staff_disciplinary_types WHERE id = $1",
                        id
                    )
                    .fetch_one(&state.pool)
                    .await
                    .map_err(Error::new)?
                    .count
                    .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Update entry
                    sqlx::query!(
                        "UPDATE staff_disciplinary_types SET name = $1, description = $2, self_assignable = $3, perm_limits = $4, additory = $5, needs_approval = $6, max_expiry = make_interval(secs => $7) WHERE id = $8",
                        name,
                        description,
                        self_assignable,
                        &perm_limits,
                        additory,
			            needs_approval,
			            max_expiry,
			            id,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                StaffDisciplinaryTypeAction::DeleteDisciplinaryType { id } => {
                    if !perms::has_perm(&user_perms, &"staff_disciplinary_types.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete staff disciplinary types [staff_disciplinary_types.delete]".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!(
                        "SELECT COUNT(*) FROM staff_disciplinary_types WHERE id = $1",
                        id
                    )
                    .fetch_one(&state.pool)
                    .await
                    .map_err(Error::new)?
                    .count
                    .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Delete entry
                    sqlx::query!("DELETE FROM staff_disciplinary_types WHERE id = $1", id)
                        .execute(&state.pool)
                        .await
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateVoteCreditTiers {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            match action {
                VoteCreditTierAction::ListTiers => {
                    let rows = sqlx::query!(
                        "SELECT id, target_type, position, cents, votes, created_at FROM vote_credit_tiers ORDER BY position ASC"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut entries = Vec::new();

                    for row in rows {
                        entries.push(VoteCreditTier {
                            id: row.id,
                            target_type: row.target_type,
                            position: row.position,
                            cents: row.cents,
                            votes: row.votes,
                            created_at: row.created_at,
                        });
                    }

                    Ok((StatusCode::OK, Json(entries)).into_response())
                }
                VoteCreditTierAction::CreateTier {
                    id,
                    position,
                    target_type,
                    cents,
                    votes,
                } => {
                    if !perms::has_perm(&user_perms, &"vote_credit_tiers.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create vote credit tiers [vote_credit_tiers.create]".to_string(),
                        )
                            .into_response());
                    }

                    if cents < 0.0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Cents cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    if votes < 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Votes cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    if target_type != "bot" && target_type != "server" {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Target type must be either 'bot' or 'server'".to_string(),
                        )
                            .into_response());
                    }

                    // Insert entry
                    let mut tx = state.pool.begin().await.map_err(Error::new)?;
                    sqlx::query!(
                        "INSERT INTO vote_credit_tiers (id, target_type, position, cents, votes) VALUES ($1, $2, $3, $4, $5)",
                        id,
                        target_type,
                        position,
                        cents,
                        votes,
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    // Now keep shifting positions until they are all unique
                    let mut index_a = position;

                    loop {
                        let rows = sqlx::query!(
                            "SELECT id, position FROM vote_credit_tiers WHERE position = $1 AND id != $2",
                            index_a,
                            id,
                        )
                        .fetch_all(&mut *tx)
                        .await
                        .map_err(Error::new)?;

                        if rows.is_empty() {
                            break;
                        }

                        let mut index_b = index_a + 1;

                        for row in rows {
                            sqlx::query!(
                                "UPDATE vote_credit_tiers SET position = $1 WHERE id = $2",
                                index_b,
                                row.id,
                            )
                            .execute(&mut *tx)
                            .await
                            .map_err(Error::new)?;

                            index_b += 1;
                        }

                        index_a = index_b;
                    }

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                VoteCreditTierAction::EditTier {
                    id,
                    position,
                    target_type,
                    cents,
                    votes,
                } => {
                    if !perms::has_perm(&user_perms, &"vote_credit_tiers.update".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update vote credit tiers [vote_credit_tiers.update]".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same id
                    if sqlx::query!("SELECT COUNT(*) FROM vote_credit_tiers WHERE id = $1", id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(Error::new)?
                        .count
                        .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    if cents < 0.0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Cents cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    if votes < 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Votes cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    if target_type != "bot" && target_type != "server" {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Target type must be either 'bot' or 'server'".to_string(),
                        )
                            .into_response());
                    }

                    let mut tx = state.pool.begin().await.map_err(Error::new)?;

                    // Update entry
                    sqlx::query!(
                        "UPDATE vote_credit_tiers SET position = $1, target_type = $2, cents = $3, votes = $4 WHERE id = $5",
                        position,
                        target_type,
                        cents,
                        votes,
                        id,
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    // Now keep shifting positions until they are all unique
                    let mut index_a = position;

                    loop {
                        let rows = sqlx::query!(
                            "SELECT id, position FROM vote_credit_tiers WHERE position = $1 AND id != $2",
                            index_a,
                            id,
                        )
                        .fetch_all(&mut *tx)
                        .await
                        .map_err(Error::new)?;

                        if rows.is_empty() {
                            break;
                        }

                        let mut index_b = index_a + 1;

                        for row in rows {
                            sqlx::query!(
                                "UPDATE vote_credit_tiers SET position = $1 WHERE id = $2",
                                index_b,
                                row.id,
                            )
                            .execute(&mut *tx)
                            .await
                            .map_err(Error::new)?;

                            index_b += 1;
                        }

                        index_a = index_b;
                    }

                    tx.commit().await.map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                VoteCreditTierAction::DeleteTier { id } => {
                    if !perms::has_perm(&user_perms, &"vote_credit_tiers.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete vote credit tiers [vote_credit_tiers.delete]".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!("SELECT COUNT(*) FROM vote_credit_tiers WHERE id = $1", id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(Error::new)?
                        .count
                        .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Delete entry
                    sqlx::query!("DELETE FROM vote_credit_tiers WHERE id = $1", id)
                        .execute(&state.pool)
                        .await
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateShopItems {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            match action {
                ShopItemAction::List => {
                    let rows = sqlx::query!(
                        "SELECT id, name, cents, target_types, benefits, created_at, last_updated, created_by, updated_by, duration, description FROM shop_items ORDER BY created_at DESC"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut entries = Vec::new();

                    for row in rows {
                        entries.push(ShopItem {
                            id: row.id,
                            name: row.name,
                            cents: row.cents,
                            target_types: row.target_types,
                            benefits: row.benefits,
                            created_at: row.created_at,
                            last_updated: row.last_updated,
                            created_by: row.created_by,
                            updated_by: row.updated_by,
                            duration: row.duration,
                            description: row.description,
                        });
                    }

                    Ok((StatusCode::OK, Json(entries)).into_response())
                }
                ShopItemAction::Create {
                    id,
                    name,
                    cents,
                    target_types,
                    benefits,
                    duration,
                    description,
                } => {
                    if !perms::has_perm(&user_perms, &"shop_items.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create shop items [shop_items.create]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    if cents < 0.0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Cents cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    if duration < 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Duration cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    // Insert entry
                    sqlx::query!(
                        "INSERT INTO shop_items (id, name, cents, target_types, benefits, created_by, updated_by, duration, description) VALUES ($1, $2, $3, $4, $5, $6, $6, $7, $8)",
                        id,
                        name,
                        cents,
                        &target_types,
                        &benefits,
                        &auth_data.user_id,
                        duration,
                        description,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                ShopItemAction::Edit {
                    id,
                    name,
                    cents,
                    target_types,
                    benefits,
                    duration,
                    description,
                } => {
                    if !perms::has_perm(&user_perms, &"shop_items.update".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update shop items [shop_items.update]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    if cents < 0.0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Cents cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    if duration < 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Duration cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same id
                    if sqlx::query!("SELECT COUNT(*) FROM shop_items WHERE id = $1", id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(Error::new)?
                        .count
                        .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Update entry
                    sqlx::query!(
                        "UPDATE shop_items SET name = $1, cents = $2, target_types = $3, benefits = $4, last_updated = NOW(), updated_by = $5, duration = $6, description = $7 WHERE id = $8",
                        name,
                        cents,
                        &target_types,
                        &benefits,
                        &auth_data.user_id,
                        duration,
                        description,
                        id,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                ShopItemAction::Delete { id } => {
                    if !perms::has_perm(&user_perms, &"shop_items.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete shop items [shop_items.delete]"
                                .to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!("SELECT COUNT(*) FROM shop_items WHERE id = $1", id)
                        .fetch_one(&state.pool)
                        .await
                        .map_err(Error::new)?
                        .count
                        .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Delete entry
                    sqlx::query!("DELETE FROM shop_items WHERE id = $1", id)
                        .execute(&state.pool)
                        .await
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateBotWhitelist {
            login_token,
            action,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
                .await
                .map_err(Error::new)?
                .resolve();

            match action {
                BotWhitelistAction::List => {
                    let rows = sqlx::query!(
                        "SELECT bot_id, user_id, reason, created_at FROM bot_whitelist ORDER BY created_at DESC"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut entries = Vec::new();

                    for row in rows {
                        entries.push(BotWhitelist {
                            bot_id: row.bot_id,
                            user_id: row.user_id,
                            reason: row.reason,
                            created_at: row.created_at,
                        });
                    }

                    Ok((StatusCode::OK, Json(entries)).into_response())
                }
                BotWhitelistAction::Add { bot_id, reason } => {
                    if !perms::has_perm(&user_perms, &"bot_whitelist.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to add to the bot whitelist (bot_whitelist.create)".to_string(),
                        )
                            .into_response());
                    }

                    // Insert entry
                    sqlx::query!(
                        "INSERT INTO bot_whitelist (user_id, bot_id, reason) VALUES ($1, $2, $3)",
                        &auth_data.user_id,
                        bot_id,
                        reason,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                BotWhitelistAction::Edit { bot_id, reason } => {
                    if !perms::has_perm(&user_perms, &"bot_whitelist.update".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update bot whitelist (bot_whitelist.update)".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!(
                        "SELECT COUNT(*) FROM bot_whitelist WHERE bot_id = $1",
                        bot_id
                    )
                    .fetch_one(&state.pool)
                    .await
                    .map_err(Error::new)?
                    .count
                    .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Update entry
                    sqlx::query!(
                        "UPDATE bot_whitelist SET reason = $1 WHERE bot_id = $2",
                        reason,
                        bot_id,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                BotWhitelistAction::Delete { bot_id } => {
                    if !perms::has_perm(&user_perms, &"bot_whitelist.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete bot whitelist entries (bot_whitelist.delete)".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!(
                        "SELECT COUNT(*) FROM bot_whitelist WHERE bot_id = $1",
                        bot_id
                    )
                    .fetch_one(&state.pool)
                    .await
                    .map_err(Error::new)?
                    .count
                    .unwrap_or(0)
                        == 0
                    {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Entry with same id does not already exist".to_string(),
                        )
                            .into_response());
                    }

                    // Delete entry
                    sqlx::query!("DELETE FROM bot_whitelist WHERE bot_id = $1", bot_id)
                        .execute(&state.pool)
                        .await
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::PopplioStaff {
            login_token,
            path,
            method,
            body,
        } => {
            let auth_data = super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let client = reqwest::Client::new();

            let Ok(method) = reqwest::Method::from_bytes(&method.into_bytes()) else {
                return Ok((StatusCode::BAD_REQUEST, "Invalid method".to_string()).into_response());
            };

            if !path.starts_with('/') {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Path must start with /".to_string(),
                )
                    .into_response());
            }

            let query = sqlx::query!(
                "SELECT popplio_token FROM staffpanel__authchain WHERE token = $1",
                login_token
            )
            .fetch_one(&state.pool)
            .await
            .map_err(Error::new)?;

            let res = client
                .request(method, crate::config::CONFIG.popplio_url.clone() + &path)
                .header("User-Agent", "arcadia")
                .header("X-Forwarded-For", &path)
                .header("X-Staff-Auth-Token", &query.popplio_token)
                .header("X-User-ID", &auth_data.user_id)
                .body(body)
                .send()
                .await
                .map_err(Error::new)?;

            let status = res.status();
            let resp = res.text().await.map_err(Error::new)?;

            Ok((status, resp).into_response())
        }
    }
}

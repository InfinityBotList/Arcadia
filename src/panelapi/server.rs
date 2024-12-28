use std::str::FromStr;
use std::sync::Arc;

use crate::impls::link::Link;
use crate::impls::{target_types::TargetType, utils::get_user_perms};
use crate::panelapi::panel_query::PanelQuery;
use crate::panelapi::types::staff_disciplinary::StaffDisciplinaryType;
use crate::panelapi::types::{
    auth::AuthorizeAction,
    blog::{BlogAction, BlogPost},
    bot_whitelist::{BotWhitelist, BotWhitelistAction},
    entity::{PartialBot, PartialEntity},
    partners::{CreatePartner, PartnerAction},
    rpc::RPCWebAction,
    rpclogs::RPCLogEntry,
    shop_items::{
        ShopCoupon, ShopCouponAction, ShopItem, ShopItemAction, ShopItemBenefit,
        ShopItemBenefitAction,
    },
    staff_disciplinary::StaffDisciplinaryTypeAction,
    vote_credit_tiers::{VoteCreditTier, VoteCreditTierAction},
    webcore::InstanceConfig,
};
use crate::rpc::core::{RPCHandle, RPCMethod};
use axum::extract::DefaultBodyLimit;
use axum::http::HeaderMap;
use axum::Json;
use kittycat::perms::{self, Permission};

use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{extract::State, http::StatusCode, Router};
use log::info;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};

use super::core::{AppState, Error};
use super::types::staff_members::StaffMemberAction;
use super::types::staff_positions::StaffPositionAction;
use crate::impls::dovewing::DovewingSource;
use strum::VariantNames;

use num_traits::ToPrimitive;

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
            PartnerAction,
            CreatePartner,
            AuthorizeAction,
            BlogAction,
            StaffPositionAction,
            StaffMemberAction,
            StaffDisciplinaryTypeAction,
            VoteCreditTierAction,
            ShopItem,
            ShopItemAction,
            ShopItemBenefit,
            ShopItemBenefitAction,
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

    let shared_state = Arc::new(AppState { pool, cache_http });

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

    let addr = format!("127.0.0.1:{}", crate::config::CONFIG.server_port.get());
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port");

    if let Err(e) = axum::serve(listener, app.into_make_service()).await {
        panic!("RPC server error: {}", e);
    }
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
#[axum::debug_handler]
async fn query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PanelQuery>,
) -> Result<impl IntoResponse, Error> {
    match req {
        PanelQuery::Authorize { version, action } => {
            super::actions::authorize::authorize(&state, version, action).await
        }
        PanelQuery::Hello {
            login_token,
            version,
        } => super::actions::hello::hello(&state, login_token, version).await,
        PanelQuery::BaseAnalytics { login_token } => {
            super::actions::baseanalytics::base_analytics(&state, login_token).await
        }
        PanelQuery::GetUser {
            login_token,
            user_id,
        } => super::actions::getuser::get_user(&state, login_token, user_id).await,
        PanelQuery::BotQueue { login_token } => {
            super::auth::check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let queue = sqlx::query!(
                "SELECT bot_id, client_id, last_claimed, claimed_by, type, approval_note, short,
                invite, approximate_votes, shards, library, invite_clicks, clicks, servers
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
                    votes: bot.approximate_votes,
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
            super::actions::searchentitys::search_entitys(&state, login_token, target_type, query)
                .await
        }
        PanelQuery::UpdatePartners {
            login_token,
            action,
        } => super::actions::updatepartners::update_partners(&state, login_token, action).await,
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
            super::actions::updatestaffposition::update_staff_position(&state, login_token, action)
                .await
        }
        PanelQuery::UpdateStaffMembers {
            login_token,
            action,
        } => {
            super::actions::updatestaffmembers::update_staff_members(&state, login_token, action)
                .await
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

                    for benefit in &benefits {
                        let rows = sqlx::query!(
                            "SELECT COUNT(*) FROM shop_item_benefits WHERE id = $1",
                            benefit
                        )
                        .fetch_one(&state.pool)
                        .await
                        .map_err(Error::new)?;

                        if rows.count.unwrap_or(0) == 0 {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                format!("Benefit {} does not exist", benefit),
                            )
                                .into_response());
                        }
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

                    for benefit in &benefits {
                        let rows = sqlx::query!(
                            "SELECT COUNT(*) FROM shop_item_benefits WHERE id = $1",
                            benefit
                        )
                        .fetch_one(&state.pool)
                        .await
                        .map_err(Error::new)?;

                        if rows.count.unwrap_or(0) == 0 {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                format!("Benefit {} does not exist", benefit),
                            )
                                .into_response());
                        }
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
        PanelQuery::UpdateShopItemBenefits {
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
                ShopItemBenefitAction::List => {
                    let rows = sqlx::query!(
                        "SELECT id, name, description, target_types, created_at, created_by, last_updated, updated_by FROM shop_item_benefits ORDER BY created_at DESC"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut entries = Vec::new();

                    for row in rows {
                        entries.push(ShopItemBenefit {
                            id: row.id,
                            name: row.name,
                            description: row.description,
                            target_types: row.target_types,
                            created_at: row.created_at,
                            created_by: row.created_by,
                            last_updated: row.last_updated,
                            updated_by: row.updated_by,
                        });
                    }

                    Ok((StatusCode::OK, Json(entries)).into_response())
                }
                ShopItemBenefitAction::Create {
                    id,
                    name,
                    description,
                    target_types,
                } => {
                    if !perms::has_perm(&user_perms, &"shop_item_benefits.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create shop item benefits [shop_item_benefits.create]".to_string(),
                        )
                            .into_response());
                    }

                    // Insert entry
                    sqlx::query!(
                        "INSERT INTO shop_item_benefits (id, name, description, target_types, created_by, updated_by) VALUES ($1, $2, $3, $4, $5, $6)",
                        id,
                        name,
                        description,
                        &target_types,
                        &auth_data.user_id,
                        &auth_data.user_id,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                ShopItemBenefitAction::Edit {
                    id,
                    name,
                    description,
                    target_types,
                } => {
                    if !perms::has_perm(&user_perms, &"shop_item_benefits.update".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update shop item benefits [shop_item_benefits.update]".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same id
                    if sqlx::query!("SELECT COUNT(*) FROM shop_item_benefits WHERE id = $1", id)
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
                        "UPDATE shop_item_benefits SET name = $1, description = $2, last_updated = NOW(), updated_by = $3, target_types = $4 WHERE id = $5",
                        name,
                        description,
                        &auth_data.user_id,
                        &target_types,
                        id,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                ShopItemBenefitAction::Delete { id } => {
                    if !perms::has_perm(&user_perms, &"shop_item_benefits.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete shop item benefits [shop_item_benefits.delete]".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!("SELECT COUNT(*) FROM shop_item_benefits WHERE id = $1", id)
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

                    // Check for shop items with this benefit
                    if sqlx::query!(
                        "SELECT COUNT(*) FROM shop_items WHERE $1 = ANY(benefits)",
                        id
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
                            "Cannot delete benefit as it is used by shop items".to_string(),
                        )
                            .into_response());
                    }

                    // Delete entry
                    sqlx::query!("DELETE FROM shop_item_benefits WHERE id = $1", id)
                        .execute(&state.pool)
                        .await
                        .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
            }
        }
        PanelQuery::UpdateShopCoupons {
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
                ShopCouponAction::List => {
                    if !perms::has_perm(&user_perms, &"shop_coupons.list".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to list shop coupons [shop_coupons.list]",
                        )
                            .into_response());
                    }

                    let rows = sqlx::query!(
                        "SELECT id, code, public, max_uses, created_at, created_by, last_updated, updated_by, reuse_wait_duration, expiry, applicable_items, cents, requirements, allowed_users, usable, target_types FROM shop_coupons ORDER BY created_at DESC"
                    )
                    .fetch_all(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    let mut entries = Vec::new();

                    for row in rows {
                        entries.push(ShopCoupon {
                            id: row.id,
                            code: row.code,
                            public: row.public,
                            max_uses: row.max_uses,
                            created_at: row.created_at,
                            created_by: row.created_by,
                            last_updated: row.last_updated,
                            updated_by: row.updated_by,
                            reuse_wait_duration: row.reuse_wait_duration,
                            expiry: row.expiry,
                            applicable_items: row.applicable_items,
                            cents: row.cents,
                            requirements: row.requirements,
                            allowed_users: row.allowed_users,
                            usable: row.usable,
                            target_types: row.target_types,
                        });
                    }

                    Ok((StatusCode::OK, Json(entries)).into_response())
                }
                ShopCouponAction::Create {
                    id,
                    code,
                    public,
                    max_uses,
                    reuse_wait_duration,
                    expiry,
                    applicable_items,
                    cents,
                    requirements,
                    allowed_users,
                    usable,
                    target_types,
                } => {
                    if !perms::has_perm(&user_perms, &"shop_coupons.create".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to create shop coupons [shop_coupons.create]".to_string(),
                        )
                            .into_response());
                    }

                    if max_uses.unwrap_or_default() <= 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Max uses must be greater than 0".to_string(),
                        )
                            .into_response());
                    }

                    if reuse_wait_duration.unwrap_or_default() <= 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Reuse wait duration must be greater than 0".to_string(),
                        )
                            .into_response());
                    }

                    if expiry.unwrap_or_default() <= 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Expiry must be greater than 0".to_string(),
                        )
                            .into_response());
                    }

                    if cents.unwrap_or_default() < 0.0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Cents cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    for item in &applicable_items {
                        let rows =
                            sqlx::query!("SELECT COUNT(*) FROM shop_items WHERE id = $1", item)
                                .fetch_one(&state.pool)
                                .await
                                .map_err(Error::new)?;

                        if rows.count.unwrap_or(0) == 0 {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                format!("Item {:#?} does not exist", item),
                            )
                                .into_response());
                        }
                    }

                    // Insert entry
                    sqlx::query!(
                        "INSERT INTO shop_coupons (id, code, public, max_uses, created_by, updated_by, reuse_wait_duration, expiry, applicable_items, cents, requirements, allowed_users, usable, target_types) VALUES ($1, $2, $3, $4, $5, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                        id,
                        code,
                        public,
                        max_uses,
                        &auth_data.user_id,
                        reuse_wait_duration,
                        expiry,
                        &applicable_items,
                        cents,
                        &requirements,
                        &allowed_users,
                        usable,
                        &target_types
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                ShopCouponAction::Edit {
                    id,
                    code,
                    public,
                    max_uses,
                    reuse_wait_duration,
                    expiry,
                    applicable_items,
                    cents,
                    requirements,
                    allowed_users,
                    usable,
                    target_types,
                } => {
                    if !perms::has_perm(&user_perms, &"shop_coupons.update".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to update shop coupons [shop_coupons.update]".to_string(),
                        )
                            .into_response());
                    }

                    if max_uses.unwrap_or_default() <= 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Max uses must be greater than 0".to_string(),
                        )
                            .into_response());
                    }

                    if reuse_wait_duration.unwrap_or_default() <= 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Reuse wait duration must be greater than 0".to_string(),
                        )
                            .into_response());
                    }

                    if expiry.unwrap_or_default() <= 0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Expiry must be greater than 0".to_string(),
                        )
                            .into_response());
                    }

                    if cents.unwrap_or_default() < 0.0 {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Cents cannot be lower than 0".to_string(),
                        )
                            .into_response());
                    }

                    for item in &applicable_items {
                        let rows =
                            sqlx::query!("SELECT COUNT(*) FROM shop_items WHERE id = $1", item)
                                .fetch_one(&state.pool)
                                .await
                                .map_err(Error::new)?;

                        if rows.count.unwrap_or(0) == 0 {
                            return Ok((
                                StatusCode::BAD_REQUEST,
                                format!("Item {:#?} does not exist", item),
                            )
                                .into_response());
                        }
                    }

                    // Insert entry
                    sqlx::query!(
                        "UPDATE shop_coupons SET code = $1, public = $2, max_uses = $3, reuse_wait_duration = $4, expiry = $5, applicable_items = $6, cents = $7, requirements = $8, updated_by = $9, last_updated = NOW(), allowed_users = $10, usable = $11, target_types = $12 WHERE id = $13",
                        code,
                        public,
                        max_uses,
                        reuse_wait_duration,
                        expiry,
                        &applicable_items,
                        cents,
                        &requirements,
                        &auth_data.user_id,
                        &allowed_users,
                        usable,
                        &target_types,
                        id,
                    )
                    .execute(&state.pool)
                    .await
                    .map_err(Error::new)?;

                    Ok((StatusCode::NO_CONTENT, "").into_response())
                }
                ShopCouponAction::Delete { id } => {
                    if !perms::has_perm(&user_perms, &"shop_coupons.delete".into()) {
                        return Ok((
                            StatusCode::FORBIDDEN,
                            "You do not have permission to delete shop coupons [shop_coupons.delete]".to_string(),
                        )
                            .into_response());
                    }

                    // Check if entry already exists with same vesion
                    if sqlx::query!("SELECT COUNT(*) FROM shop_coupons WHERE id = $1", id)
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
                    sqlx::query!("DELETE FROM shop_coupons WHERE id = $1", id)
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
    }
}

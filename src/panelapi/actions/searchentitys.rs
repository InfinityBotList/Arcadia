use crate::impls::dovewing::{get_platform_user, DovewingSource};
use crate::impls::target_types::TargetType;
use crate::impls::utils::get_entity_managers;
use crate::panelapi::auth::check_auth;
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::entity::{PartialBot, PartialEntity, PartialServer};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

pub async fn search_entitys(
    state: &AppState,
    login_token: String,
    target_type: TargetType,
    query: String,
) -> Result<Response, Error> {
    check_auth(&state.pool, &login_token)
        .await
        .map_err(Error::new)?;

    match target_type {
        TargetType::Bot => {
            let queue = sqlx::query!(
            "
            SELECT bot_id, client_id, type, approximate_votes, shards, library, invite_clicks, clicks,
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
                let owners = get_entity_managers(TargetType::Bot, &bot.bot_id, &state.pool)
                    .await
                    .map_err(Error::new)?;

                let user = get_platform_user(
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
                    votes: bot.approximate_votes,
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
            SELECT server_id, name, total_members, online_members, short, type, approximate_votes, invite_clicks,
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
                let owners =
                    get_entity_managers(TargetType::Server, &server.server_id, &state.pool)
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
                    votes: server.approximate_votes,
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

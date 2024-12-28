use crate::panelapi::auth::check_auth;
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::analytics::BaseAnalytics;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

pub async fn base_analytics(state: &AppState, login_token: String) -> Result<Response, Error> {
    check_auth(&state.pool, &login_token)
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
                .map(|t| {
                    (
                        if t.open {
                            "open".to_string()
                        } else {
                            "closed".to_string()
                        },
                        t.count.unwrap_or_default(),
                    )
                })
                .collect(),
            total_users: total_users.count.unwrap_or_default(),
            changelogs_count: total_changelogs.count.unwrap_or_default(),
        }),
    )
        .into_response())
}

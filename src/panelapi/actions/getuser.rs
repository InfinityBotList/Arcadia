use crate::impls::dovewing::{get_platform_user, DovewingSource};
use crate::panelapi::auth::check_auth;
use crate::panelapi::core::{AppState, Error};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

pub async fn get_user(
    state: &AppState,
    login_token: String,
    user_id: String,
) -> Result<Response, Error> {
    check_auth(&state.pool, &login_token)
        .await
        .map_err(Error::new)?;

    let user = get_platform_user(
        &state.pool,
        DovewingSource::Discord(state.cache_http.clone()),
        &user_id,
    )
    .await
    .map_err(Error::new)?;

    Ok((StatusCode::OK, Json(user)).into_response())
}

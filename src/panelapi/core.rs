use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt::Display;

pub struct Error {
    pub status: StatusCode,
    pub message: String,
}

impl Error {
    pub fn new(e: impl Display) -> Self {
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
    pub pool: sqlx::PgPool,
}

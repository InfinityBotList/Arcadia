use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use axum_macros::debug_handler;
use log::info;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use ts_rs::TS;

use crate::{
    authkit,
    mongoschemas::{core::Model, internal_cases::InternalCases},
    Data,
};

struct AppState {
    data: Data,
}

pub enum AshfurResponse {
    Content(String),
    NoContent,
}

impl IntoResponse for AshfurResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Content(content) => (StatusCode::OK, content).into_response(),
            Self::NoContent => (StatusCode::NO_CONTENT, "").into_response(),
        }
    }
}

pub async fn init(data: Data) {
    let shared_state = Arc::new(AppState { data });

    let app = Router::new()
        .route("/query", post(query))
        .with_state(shared_state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = "127.0.0.1:4859"
        .parse()
        .expect("Invalid RPC server address");

    info!("Starting RPC server on {}", addr);

    if let Err(e) = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
    {
        panic!("Axum server error: {}", e);
    }
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = ".generated/AshfurQuery.ts")]
pub struct AshfurQuery {
    pub auth: authkit::AuthPayload,
    pub query: QueryInner,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = ".generated/QueryInner.ts")]
pub enum QueryInner {
    /// Test query
    Test {
        /// Echo string
        echo: String,
    },
    /// Internal cases, filtered by a user id
    InternalCasesFilterByUserId {
        user_id: String,
    }
}

#[debug_handler]
async fn query(
    State(state): State<Arc<AppState>>,
    Json(query): Json<AshfurQuery>,
) -> Result<AshfurResponse, (StatusCode, String)> {
    query
        .auth
        .authorize(&state.data)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    match query.query {
        QueryInner::Test { echo } => Ok(AshfurResponse::Content(echo.to_string())),
        QueryInner::InternalCasesFilterByUserId { user_id } => {
            let cases = InternalCases::get(&state.data, doc! { "user": user_id }, None)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            Ok(AshfurResponse::Content(
                serde_json::to_string(&cases).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to serialize cases: {}", e),
                    )
                })?,
            ))
        }
    }
}

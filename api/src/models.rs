use serde::{Serialize, Deserialize};

pub struct AppState {
    pub pool: sqlx::PgPool
}

#[derive(Serialize, Deserialize)]
pub struct APIResponse {
    pub done: bool,
    pub reason: String,
    pub context: Option<String>,
}
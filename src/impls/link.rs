use ts_rs::TS;
use utoipa::ToSchema;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, TS, Clone, Default, ToSchema)]
pub struct Link {
    pub name: String,
    pub value: String,
}

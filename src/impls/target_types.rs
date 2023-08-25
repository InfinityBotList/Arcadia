use std::fmt::{Formatter, Display};

use strum_macros::{EnumString, EnumVariantNames};
use ts_rs::TS;
use utoipa::ToSchema;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, EnumString, ToSchema, TS, EnumVariantNames, Clone)]
#[ts(export, export_to = ".generated/TargetType.ts")]
pub enum TargetType {
    Bot,
    Server,
    Team,
    Pack,
}

impl Display for TargetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Bot => write!(f, "bot"),
            TargetType::Server => write!(f, "server"),
            TargetType::Team => write!(f, "team"),
            TargetType::Pack => write!(f, "pack"),
        }
    }
}

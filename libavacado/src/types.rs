use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use utoipa::ToSchema;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub valid: bool,
    pub bot: bool,
}

impl DiscordUser {
    pub fn from_user(user: serenity::model::user::User) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.name.clone(),
            discriminator: user.discriminator.to_string(),
            avatar: user.avatar_url(),
            valid: true,
            bot: user.bot,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct StaffAppQuestion {
    pub id: String,
    pub question: String,
    pub para: String,
    pub placeholder: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct StaffPosition {
    pub info: String,
    pub open: bool,
    pub needs_interview: bool,
    pub name: String,
    pub questions: Vec<StaffAppQuestion>,
    pub app_site_rendered: bool,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct StaffAppData {
    pub positions: Vec<String>,
    pub staff: StaffPosition,
    pub dev: StaffPosition,
    pub certification: StaffPosition, // TBD whether it will be on app site or main site
    pub partners: StaffPosition,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct StaffAppResponse {
    pub app_id: String,
    pub user_id: String,
    pub answers: JsonValue,
    pub interview: JsonValue,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub likes: Vec<String>,
    pub dislikes: Vec<String>,
    pub position: String,
}

impl StaffAppData {
    // Ensure all positions have a function in this impl
    pub fn staff_questions(&self, position: &str) -> &StaffPosition {
        match position {
            "staff" => &self.staff,
            "dev" => &self.dev,
            "certification" => &self.certification,
            "partners" => &self.partners,
            _ => panic!("Invalid position"),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Link {
    pub name: String,
    pub value: String,
}

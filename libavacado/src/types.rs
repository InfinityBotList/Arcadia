use std::{sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Debug)]
pub struct Search {
    pub bots: Vec<SearchBot>,
    pub packs: Vec<SearchPack>,
    pub users: Vec<SearchUser>,
}

#[derive(Serialize, Debug)]
pub struct SearchBot {
    pub user: Arc<DiscordUser>,
    pub tags: Vec<String>,
    pub description: String,
    pub invite: String,
    pub servers: i32,
    pub shards: i32,
    pub votes: i32,
    pub certified: bool,
    pub r#type: String,
    pub banner: Option<String>,
    pub invite_clicks: i32,
    pub clicks: i32,
    pub vanity: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct SearchPack {
    pub name: String,
    pub url: String,
    pub description: String,
    pub bots: Vec<SearchBot>,
    pub votes: i64,
}

#[derive(Serialize, Debug)]
pub struct SearchUser {
    pub user: Arc<DiscordUser>,
    pub about: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub valid: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StaffAppQuestion {
    pub id: String,
    pub question: String,
    pub para: String,
    pub placeholder: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StaffPosition {
    pub info: String,
    pub open: bool,
    pub needs_interview: bool,
    pub name: String,
    pub questions: Vec<StaffAppQuestion>,
    pub app_site_rendered: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StaffAppData {
    pub positions: Vec<String>,
    pub staff: StaffPosition,
    pub dev: StaffPosition,
    pub certification: StaffPosition, // TBD whether it will be on app site or main site
    pub partners: StaffPosition,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Deserialize)]
pub struct CreateBot {
    pub bot_id: String,
    pub short: String,
    pub long: String,
    pub prefix: String,
    pub invite: String,
    pub support: String,
    pub website: String,
    pub github: String,
    pub library: String,
    pub donate: String,
    pub tags: Vec<String>,
    pub nsfw: bool,
    pub cross_add: bool,
    pub additional_owners: Vec<String>,
    pub staff_note: String,
    pub background: String,
}
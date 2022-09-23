use std::sync::Arc;

use serde::{Deserialize, Serialize};

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
}

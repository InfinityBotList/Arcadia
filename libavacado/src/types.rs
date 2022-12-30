use serde::{Deserialize, Serialize};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Deserialize, Debug)]
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
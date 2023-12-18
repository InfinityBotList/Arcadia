use once_cell::sync::Lazy;
use poise::serenity_prelude::{ChannelId, GuildId, RoleId, UserId};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Write};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::Error;

/// Global config object
pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::load().expect("Failed to load config"));

#[derive(Serialize, Deserialize, Clone)]
pub struct Servers {
    pub main: GuildId,
    pub staff: GuildId,
    pub testing: GuildId,
}

impl Default for Servers {
    fn default() -> Self {
        Self {
            main: GuildId::new(758641373074423808),
            staff: GuildId::new(870950609291972618),
            testing: GuildId::new(870952645811134475),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Roles {
    pub developer: RoleId,
    pub head_developer: RoleId,
    pub staff_manager: RoleId,
    pub head_manager: RoleId,
    pub web_moderator: RoleId,
    pub owner: RoleId,
    pub awaiting_staff: RoleId,
    pub bot_developer: RoleId,
    pub certified_developer: RoleId,
    pub bot_role: RoleId,
    pub bug_hunters: RoleId,
}

impl Default for Roles {
    fn default() -> Self {
        Self {
            developer: RoleId::new(870950609291972625),
            head_developer: RoleId::new(870950609317150732),
            staff_manager: RoleId::new(870950609291972626),
            head_manager: RoleId::new(870950609291972627),
            web_moderator: RoleId::new(870950609291972622),
            owner: RoleId::new(870950609317150734),
            awaiting_staff: RoleId::new(1029058929361174678),
            bot_developer: RoleId::new(758756147313246209),
            certified_developer: RoleId::new(759468303344992266),
            bot_role: RoleId::new(758652296459976715),
            bug_hunters: RoleId::new(1042546603795427398),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Channels {
    /// The testing lounge channel where autounclaims are sent
    pub testing_lounge: ChannelId,
    pub mod_logs: ChannelId,
    // System channel
    pub system: ChannelId,
    pub uptime: ChannelId,
    pub staff_logs: ChannelId,
}

impl Default for Channels {
    fn default() -> Self {
        Self {
            testing_lounge: ChannelId::new(891611731699335209),
            mod_logs: ChannelId::new(911907978926493716),
            system: ChannelId::new(762958420277067786),
            uptime: ChannelId::new(1083108330442076292),
            staff_logs: ChannelId::new(1186195848497999912),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct PanelConfig {
    /// Discord client ID for panel login app
    pub client_id: String,
    /// Discord client secret for panel login app
    pub client_secret: String,
    /// Redirect URL for panel login app
    pub redirect_url: Vec<String>,

    /// CDN scopes for the panel API (locations for the CDN)
    ///
    /// Currently the panel uses the following scopes:
    /// - ibl@main
    pub cdn_scopes: HashMap<String, CdnScopeData>,
    /// Main scope
    pub main_scope: String,
}

#[derive(Serialize, Deserialize, TS, ToSchema, Clone, Default)]
#[ts(export, export_to = ".generated/CdnScopeData.ts")]
pub struct CdnScopeData {
    /// Path in local fs (or remote if support is added)
    pub path: String,
    /// Exposed URL for the CDN
    pub exposed_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub token: String,
    pub servers: Servers,
    pub roles: Roles,
    pub channels: Channels,
    pub frontend_url: String,
    pub infernoplex_url: String,
    pub htmlsanitize_url: String,
    pub popplio_url: String,
    pub cdn_url: String,
    pub proxy_url: String,
    pub owners: Vec<UserId>,
    pub protected_bots: Vec<UserId>,
    pub panel: PanelConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: String::from(""),
            token: String::from(""),
            servers: Servers::default(),
            roles: Roles::default(),
            channels: Channels::default(),
            frontend_url: String::from("https://infinitybots.gg"),
            infernoplex_url: String::from("https://infernoplex.infinitybots.gg"),
            popplio_url: String::from("https://spider-staging.infinitybots.gg"),
            htmlsanitize_url: String::from("https://hs.infinitybots.gg/"),
            cdn_url: String::from("https://cdn.infinitybots.gg"),
            proxy_url: String::from("http://127.0.0.1:3219"),
            owners: vec![UserId::new(510065483693817867)],
            protected_bots: vec![
                UserId::new(1019662370278228028), // Reedwhisker (PTB) - Main Bot
            ],
            panel: PanelConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Error> {
        // Delete config.yaml.sample if it exists
        if std::path::Path::new("config.yaml.sample").exists() {
            std::fs::remove_file("config.yaml.sample")?;
        }

        // Create config.yaml.sample
        let mut sample = File::create("config.yaml.sample")?;

        // Write default config to config.yaml.sample
        sample.write_all(serde_yaml::to_string(&Config::default())?.as_bytes())?;

        // Open config.yaml
        let file = File::open("config.yaml");

        match file {
            Ok(file) => {
                // Parse config.yaml
                let cfg: Config = serde_yaml::from_reader(file)?;

                // Return config
                Ok(cfg)
            }
            Err(e) => {
                // Print error
                println!("config.yaml could not be loaded: {}", e);

                // Exit
                std::process::exit(1);
            }
        }
    }
}

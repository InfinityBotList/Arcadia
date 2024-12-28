use crate::Error;
use once_cell::sync::Lazy;
use poise::serenity_prelude::{ChannelId, GuildId, RoleId, UserId};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Write};

pub static CURRENT_ENV: Lazy<&str> = Lazy::new(|| {
    let current_env = include_bytes!("../current-env");

    std::str::from_utf8(current_env).unwrap()
});

/// Global config object
pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::load().expect("Failed to load config"));

#[derive(Serialize, Deserialize, Default)]
pub struct Differs<T: Default + Clone> {
    staging: T,
    prod: T,
}

impl<T: Default + Clone> Differs<T> {
    /// Get the value for a given environment
    pub fn get_for_env(&self, env: &str) -> T {
        if env == "staging" {
            self.staging.clone()
        } else {
            self.prod.clone()
        }
    }

    /// Get the value for the current environment
    pub fn get(&self) -> T {
        self.get_for_env(*CURRENT_ENV)
    }
}

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
    pub awaiting_staff: RoleId,
    pub bot_developer: RoleId,
    pub certified_developer: RoleId,
    pub bot_role: RoleId,
    pub bug_hunters: RoleId,
    pub top_reviewers: RoleId,
}

impl Default for Roles {
    fn default() -> Self {
        Self {
            awaiting_staff: RoleId::new(1029058929361174678),
            bot_developer: RoleId::new(758756147313246209),
            certified_developer: RoleId::new(759468303344992266),
            bot_role: RoleId::new(758652296459976715),
            bug_hunters: RoleId::new(1042546603795427398),
            top_reviewers: RoleId::new(1239696066350420038),
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
    pub cdn_scopes: Differs<HashMap<String, CdnScopeData>>,

    /// Main scope
    pub main_scope: String,

    /// Panel scope, used by frontend for validation. Should be static
    pub panel_scope: String,
    /// Panel response scope, used by frontend for validation. Should be static
    pub panel_response_scope: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CdnScopeData {
    /// Path in local fs (or remote if support is added)
    pub path: String,
    /// Exposed URL for the CDN
    pub exposed_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub server_port: Differs<u16>,
    pub prefix: Differs<String>,
    pub database_url: String,
    pub token: Differs<String>,
    pub servers: Servers,
    pub roles: Roles,
    pub channels: Channels,
    pub frontend_url: Differs<String>,
    pub infernoplex_url: String,
    pub htmlsanitize_url: String,
    pub borealis_url: String,
    pub popplio_url: String,
    pub cdn_url: String,
    pub proxy_url: String,
    pub owners: Vec<UserId>,
    pub protected_bots: Vec<UserId>,
    pub panel: PanelConfig,
    pub japi_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_port: Differs {
                staging: 3011,
                prod: 3010,
            },
            prefix: Differs {
                staging: String::from("ibb!"),
                prod: String::from("ibs!"),
            },
            database_url: String::from(""),
            token: Differs {
                staging: String::from(""),
                prod: String::from(""),
            },
            servers: Servers::default(),
            roles: Roles::default(),
            channels: Channels::default(),
            frontend_url: Differs {
                staging: String::from("https://reedwhisker.infinitybots.gg"),
                prod: String::from("https://infinitybots.gg"),
            },
            infernoplex_url: String::from("https://infernoplex.infinitybots.gg"),
            borealis_url: String::from("http://localhost:2837"),
            popplio_url: String::from("https://spider-staging.infinitybots.gg"),
            htmlsanitize_url: String::from("https://hs.infinitybots.gg/"),
            cdn_url: String::from("https://cdn.infinitybots.gg"),
            proxy_url: String::from("http://127.0.0.1:3219"),
            owners: vec![UserId::new(510065483693817867)],
            protected_bots: vec![
                UserId::new(1019662370278228028), // Reedwhisker (PTB) - Main Bot
            ],
            panel: PanelConfig::default(),
            japi_key: String::from(""),
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

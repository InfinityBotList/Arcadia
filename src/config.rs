use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write, num::NonZeroU64};

use crate::Error;

/// Global config object
pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::load().expect("Failed to load config"));

#[derive(Serialize, Deserialize)]
pub struct Servers {
    pub main: NonZeroU64,
    pub staff: NonZeroU64,
    pub testing: NonZeroU64,
}

impl Default for Servers {
    fn default() -> Self {
        Self {
            main: NonZeroU64::new(758641373074423808).unwrap(),
            staff: NonZeroU64::new(870950609291972618).unwrap(),
            testing: NonZeroU64::new(870952645811134475).unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Roles {
    pub developer: NonZeroU64,
    pub head_developer: NonZeroU64,
    pub staff_manager: NonZeroU64,
    pub head_manager: NonZeroU64,
    pub web_moderator: NonZeroU64,
    pub owner: NonZeroU64,
    pub awaiting_staff: NonZeroU64,
    pub bot_developer: NonZeroU64,
    pub certified_developer: NonZeroU64,
}

impl Default for Roles {
    fn default() -> Self {
        Self {
            developer: NonZeroU64::new(870950609291972625).unwrap(),
            head_developer: NonZeroU64::new(870950609317150732).unwrap(),
            staff_manager: NonZeroU64::new(870950609291972626).unwrap(),
            head_manager: NonZeroU64::new(870950609291972627).unwrap(),
            web_moderator: NonZeroU64::new(870950609291972622).unwrap(),
            owner: NonZeroU64::new(870950609317150734).unwrap(),
            awaiting_staff: NonZeroU64::new(1029058929361174678).unwrap(),
            bot_developer: NonZeroU64::new(758756147313246209).unwrap(),
            certified_developer: NonZeroU64::new(759468303344992266).unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Channels {
    /// The testing lounge channel where autounclaims are sent
    pub testing_lounge: NonZeroU64,
    pub mod_logs: NonZeroU64,
    /// Where onboardings are sent to for staff managers to moderate
    pub onboarding_channel: NonZeroU64,
}

impl Default for Channels {
    fn default() -> Self {
        Self {
            testing_lounge: NonZeroU64::new(891611731699335209).unwrap(),
            mod_logs: NonZeroU64::new(911907978926493716).unwrap(),
            onboarding_channel: NonZeroU64::new(990716921475375114).unwrap(),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Metro {
    pub list_id: String,
    pub secret: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub token: String,
    pub servers: Servers,
    pub roles: Roles,
    pub channels: Channels,
    pub test_bot: NonZeroU64,
    pub frontend_url: String,
    pub proxy_url: String,
    pub metro: Metro,
    pub rpc_allowed_urls: Vec<String>,
    pub owners: Vec<NonZeroU64>,
    pub protected_bots: Vec<NonZeroU64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: String::from(""),
            token: String::from(""),
            servers: Servers::default(),
            roles: Roles::default(),
            channels: Channels::default(),
            metro: Metro::default(),
            test_bot: NonZeroU64::new(990885577979224104).unwrap(),
            frontend_url: String::from("https://infinitybots.gg"),
            proxy_url: String::from("http://127.0.0.1:3219"),
            rpc_allowed_urls: vec![],
            owners: vec![NonZeroU64::new(510065483693817867).unwrap()],
            protected_bots: vec![
                NonZeroU64::new(1019662370278228028).unwrap(), // Reedwhisker (PTB) - Main Bot
            ],
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

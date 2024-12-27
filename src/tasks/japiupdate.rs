/*
type japidata struct {
    Cached bool `json:"cached"`
    Data   struct {
        Message     string `json:"message,omitempty"`
        Application *struct {
            ID          string   `json:"id"`
            BotPublic   bool     `json:"bot_public"`
            Description string   `json:"description"`
            Tags        []string `json:"tags"`
        } `json:"application"`
        Bot *struct {
            ID                    string   `json:"id"`
            ApproximateGuildCount int      `json:"approximate_guild_count"`
            Username              string   `json:"username"`
            AvatarURL             string   `json:"avatarURL"`
            AvatarHash            string   `json:"avatarHash"`
            PublicFlagsArray      []string `json:"public_flags_array"`
        } `json:"bot"`
    } `json:"data"`
}
 */

use std::time::UNIX_EPOCH;

#[allow(dead_code)]
#[derive(serde::Deserialize)]
pub struct JapiData {
    pub cached: bool,
    pub data: JapiDataInner,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
pub struct JapiDataInner {
    pub message: Option<String>,
    pub application: Option<JapiDataApplication>,
    pub bot: Option<JapiDataBot>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
pub struct JapiDataApplication {
    pub id: String,
    #[serde(default)]
    pub bot_public: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
pub struct JapiDataBot {
    pub id: String,
    pub approximate_guild_count: Option<i32>,
    pub username: Option<String>,
    #[serde(rename = "avatarURL")]
    pub avatar_url: Option<String>,
    #[serde(rename = "avatarHash")]
    pub avatar_hash: Option<String>,
    pub public_flags_array: Vec<String>,
}

static REQS_MADE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static LAST_REQS_REFRESH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub async fn japi_updater(ctx: &serenity::all::Context) -> Result<(), crate::Error> {
    if LAST_REQS_REFRESH.load(std::sync::atomic::Ordering::Acquire)
        - std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs()
        >= 3600
    {
        REQS_MADE.store(0, std::sync::atomic::Ordering::Release);
        LAST_REQS_REFRESH.store(
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs(),
            std::sync::atomic::Ordering::Release,
        );
    }

    let data = ctx.data::<crate::Data>();
    let pool = &data.pool;

    let bots_to_update = sqlx::query!(
        "SELECT bot_id FROM bots WHERE (type = 'approved' OR type = 'certified') AND (last_stats_post IS NULL OR NOW() - last_stats_post > INTERVAL '3 days') AND (last_japi_update IS NULL OR NOW() - last_japi_update > INTERVAL '3 days') ORDER BY RANDOM() LIMIT 10"
    )
    .fetch_all(pool)
    .await?;

    for bot in bots_to_update {
        let bot_id = bot.bot_id;

        if REQS_MADE.fetch_add(1, std::sync::atomic::Ordering::Release) > 1800 {
            return Err("Internal error: JAPI rate limit hit".into());
        }

        let response = reqwest::get(&format!(
            "https://japi.rest/discord/v1/application/{}",
            bot_id
        ))
        .await?;

        if !response.status().is_success() {
            log::error!("Failed to fetch bot {} from JAPI", bot_id);
            continue;
        }

        let bot_data = response.json::<JapiData>().await?;

        if let Some(bot_data) = bot_data.data.bot {
            sqlx::query!(
                "UPDATE bots SET last_japi_update = NOW(), servers = $1 WHERE bot_id = $2",
                bot_data.approximate_guild_count,
                bot_id
            )
            .execute(pool)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE bots SET last_japi_update = NOW() WHERE bot_id = $1",
                bot_id
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

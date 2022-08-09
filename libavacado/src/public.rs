use std::{sync::Arc, time::Duration};

use crate::types::{DiscordUser, Error};

use deadpool_redis::redis::AsyncCommands;
use moka::future::Cache;
use serenity::model::id::UserId;
use serenity::{http::CacheHttp, model::id::GuildId};

use rand::{distributions::Alphanumeric, Rng};

// Private struct to handle rust trait errors
pub struct AvcCacheHttpImpl {
    cache: Arc<serenity::cache::Cache>,
    http: Arc<serenity::http::Http>,
}

impl CacheHttp for AvcCacheHttpImpl {
    fn http(&self) -> &serenity::http::Http {
        &self.http
    }

    fn cache(&self) -> Option<&Arc<serenity::cache::Cache>> {
        Some(&self.cache)
    }
}

// Public avacado client used to store caches
pub struct AvacadoPublic {
    pub search_cache: Cache<String, Arc<crate::types::Search>>,
    pub redis: deadpool_redis::Pool,
    pub user_cache: Cache<u64, Arc<DiscordUser>>,
    pub cache: Arc<serenity::cache::Cache>,

    // Http is unused right now but will be used later
    #[allow(dead_code)]
    pub http: Arc<serenity::http::Http>,

    // Custom struct to avoid rust trait errors
    pub cache_http: AvcCacheHttpImpl,
}

impl AvacadoPublic {
    pub fn new(cache: Arc<serenity::cache::Cache>, http: Arc<serenity::http::Http>) -> Self {
        let cfg = deadpool_redis::Config::from_url("redis://127.0.0.1:6379/8");
        Self {
            search_cache: Cache::builder()
                // Time to live (TTL): 5 minutes
                .time_to_live(Duration::from_secs(60 * 5))
                // Time to idle (TTI): 3 minutes
                .time_to_idle(Duration::from_secs(60 * 3))
                // Create the cache.
                .build(),
            user_cache: Cache::builder()
                // Time to live (TTL): 3 hours
                .time_to_live(Duration::from_secs(3 * 60 * 60))
                // Time to idle (TTI):  2 hours
                .time_to_idle(Duration::from_secs(2 * 60 * 60))
                // Create the cache.
                .build(),
            redis: cfg
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .unwrap(),
            cache: cache.clone(),
            http: http.clone(),
            cache_http: AvcCacheHttpImpl { cache, http },
        }
    }
}

pub fn gen_random(length: usize) -> String {
    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();

    s
}

pub async fn get_user(
    public: &AvacadoPublic,
    id: &str,
    no_err: bool,
) -> Result<Arc<DiscordUser>, Error> {
    let id_u64 = id.parse::<u64>()?;

    let cached = public.user_cache.get(&id_u64);

    if let Some(cached) = cached {
        return Ok(cached);
    }

    // Try fetching from redis
    let mut conn = public.redis.get().await.unwrap();

    let user_cached: String = conn.get(id).await.unwrap_or_else(|_| "".to_string());

    if !user_cached.is_empty() {
        let user: Result<DiscordUser, _> = serde_json::from_str(&user_cached);

        if let Ok(user) = user {
            let user = Arc::new(user);

            // Copy user object from redis to cache
            public.user_cache.insert(id_u64, user.clone()).await;
            return Ok(user);
        }
    }

    // Next try fetching it from main server as a member
    let main_server = std::env::var("MAIN_SERVER")?;

    let main_server_u64 = main_server.parse::<u64>()?;

    let member = public
        .cache
        .member(GuildId(main_server_u64), UserId(id_u64));

    if let Some(member) = member {
        let user = DiscordUser {
            id: member.user.id.0.to_string(),
            username: member.user.name.to_string(),
            discriminator: member.user.discriminator.to_string(),
            avatar: member.user.avatar_url(),
        };

        let arc_user = Arc::new(user);

        public.user_cache.insert(id_u64, arc_user.clone()).await;

        // Save to redis as well with a expiry
        let user_json = serde_json::to_string(&arc_user.clone())?;

        conn.set_ex(id, user_json, 60 * 60 * 4).await?;

        return Ok(arc_user);
    }

    // Not in main server, lets just get it from discord API

    let user = UserId(id_u64).to_user(&public.cache_http).await;

    if user.is_err() {
        if no_err {
            return Ok(Arc::new(DiscordUser {
                id: id.to_string(),
                username: "Unknown User".to_string(),
                discriminator: "0000".to_string(),
                avatar: None,
            }));
        } else {
            return Err(Box::new(user.unwrap_err()));
        }
    }

    let user = user.unwrap();

    let arc_user = Arc::new(DiscordUser {
        id: user.id.0.to_string(),
        username: user.name.to_string(),
        discriminator: user.discriminator.to_string(),
        avatar: user.avatar_url(),
    });

    public.user_cache.insert(id_u64, arc_user.clone()).await;

    // Save to redis as well with a expiry
    let user_json = serde_json::to_string(&arc_user.clone())?;

    conn.set_ex(id, user_json, 60 * 60 * 4).await?;

    Ok(arc_user)
}

use std::{sync::Arc, time::Duration};

use crate::types::Error;

use deadpool_redis::redis::AsyncCommands;
use moka::future::Cache;
use serde::{Serialize, Deserialize};
use serenity::model::id::UserId;
use serenity::{http::CacheHttp, model::id::GuildId};
use serenity::CacheAndHttp;
use sqlx::PgPool;

use rand::{distributions::Alphanumeric, Rng};

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

// Public avacado client used to store caches
pub struct AvacadoPublic {
    search_cache: Cache<String, Arc<Search>>,
    redis: deadpool_redis::Pool,
    user_cache: Cache<u64, Arc<DiscordUser>>,
    cache_http: Arc<CacheAndHttp>,
}

impl AvacadoPublic {
    pub fn new(cache_http: Arc<CacheAndHttp>) -> Self {
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
            redis: cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1)).unwrap(),
            cache_http,
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

pub async fn get_user(public: &AvacadoPublic, id: &str, no_err: bool) -> Result<Arc<DiscordUser>, Error> {
    let id_u64 = id.parse::<u64>()?;

    let cached = public.user_cache.get(&id_u64);

    if let Some(cached) = cached {
        return Ok(cached)
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

    let cache = public.cache_http.cache().unwrap();

    let member = cache.member(GuildId(main_server_u64), UserId(id_u64));

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

        return Ok(arc_user)
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
            }))
        } else {
            return Err(Box::new(user.unwrap_err()))
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

pub async fn search_bots(
    query: &String,
    pool: &PgPool,
    public: &AvacadoPublic
) -> Result<Arc<Search>, Error> {

    let search = public.search_cache.get(query);

    if search.is_some() {
        let search_inf = search.unwrap().clone();
        return Ok(search_inf.into());
    }

    let bots = sqlx::query!(
        "SELECT DISTINCT bot_id, name, short, invite, servers, shards, votes, certified, tags FROM (
            SELECT bot_id, owner, type, name, short, invite, servers, shards, votes, certified, tags, unnest(tags) AS tag_unnest FROM bots
        ) bots WHERE type = 'approved' AND (name ILIKE $2 OR owner @@ $1 OR short @@ $1 OR tag_unnest @@ $1) ORDER BY votes DESC, certified DESC LIMIT 6",
        query,
        "%".to_string() + query + "%"
    )
    .fetch_all(pool)
    .await?;

    let mut search_bots = Vec::new();

    for bot in bots {
        search_bots.push(SearchBot {
            user: get_user(public, &bot.bot_id, true).await?,
            description: bot.short,
            invite: bot.invite,
            servers: bot.servers,
            shards: bot.shards,
            votes: bot.votes,
            certified: bot.certified,
            tags: bot.tags,
        });
    }

    let packs = sqlx::query!(
        "SELECT DISTINCT name, short, bots, votes, url FROM (
            SELECT name, short, owner, bots, votes, url, unnest(bots) AS bot_unnest FROM packs
        ) packs WHERE (name ILIKE $2 OR bot_unnest @@ $1 OR short @@ $1 OR owner @@ $1) LIMIT 6",
        query,
        "%".to_string() + query + "%"
    )
    .fetch_all(pool)
    .await?;

    let mut search_packs = Vec::new();

    for pack in packs {
        search_packs.push(SearchPack {
            name: pack.name,
            description: pack.short,
            url: pack.url,
            bots: Vec::new(),
            votes: pack.votes
        });

        for bot in pack.bots {
            let res = sqlx::query!(
                "SELECT bot_id, name, short, invite, servers, shards, votes, certified, tags FROM bots WHERE bot_id = $1",
                bot
            )
            .fetch_one(pool)
            .await;

            if res.is_err() {
                continue
            }

            let res = res.unwrap();

            search_packs.last_mut().unwrap().bots.push(SearchBot {
                user: get_user(public, &res.bot_id, true).await?,
                description: res.short,
                invite: res.invite,
                servers: res.servers,
                shards: res.shards,
                votes: res.votes,
                certified: res.certified,
                tags: res.tags,
            });
        }
    }

    let users = sqlx::query!(
        "SELECT DISTINCT users.user_id, users.username, users.about FROM users 
        INNER JOIN bots ON bots.owner = users.user_id 
        WHERE (bots.name ILIKE $2 OR bots.short @@ $1 OR bots.bot_id @@ $1) 
        OR (users.username @@ $1) LIMIT 6",
        query,
        "%".to_string() + query + "%"
    )
    .fetch_all(pool)
    .await?;

    let mut search_users = Vec::new();

    for user in users {
        search_users.push(SearchUser {
            user: get_user(public, &user.user_id, true).await?,
            about: user.about
        });
    }

    let res = Arc::new(Search {
        bots: search_bots,
        packs: search_packs,
        users: search_users
    });

    public.search_cache.insert(query.clone(), res.clone()).await;

    Ok(res)
}
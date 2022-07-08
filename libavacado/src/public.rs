use std::{sync::Arc, time::Duration};

use crate::types::Error;

use moka::future::Cache;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
pub struct Search {
    pub bots: Vec<SearchBot>,
    pub packs: Vec<SearchPack>,
    pub users: Vec<SearchUser>,
}

#[derive(Serialize)]
pub struct SearchBot {
    pub bot_id: String,
    pub name: String,
    pub description: String,
    pub invite: String,
    pub servers: i32,
    pub shards: i32,
    pub votes: i32,
    pub certified: bool,
}

#[derive(Serialize)]
pub struct SearchPack {
    pub name: String,
    pub description: String,
    pub bots: Vec<String>,
    pub votes: i64,
}

#[derive(Serialize)]
pub struct SearchUser {
    pub user_id: String,
    pub about: Option<String>,
}

// Public avacado client used to store caches
pub struct AvacadoPublic {
    search_cache: Cache<String, Arc<Search>>
}

impl AvacadoPublic {
    pub fn new() -> Self {
        Self {
            search_cache: Cache::builder()
            // Time to live (TTL): 1 minute
            .time_to_live(Duration::from_secs(60))
            // Time to idle (TTI):  30 seconds
            .time_to_idle(Duration::from_secs(30))
            // Create the cache.
            .build(),
        }
    }
}

pub async fn search_bots(
    query: String,
    pool: &PgPool,
    public: &AvacadoPublic
) -> Result<Arc<Search>, Error> {
    let search = public.search_cache.get(&query);

    if search.is_some() {
        let search_inf = search.unwrap().clone();
        return Ok(search_inf.into());
    }

    let bots = sqlx::query!(
        "SELECT DISTINCT bot_id, name, short, invite, servers, shards, votes, certified FROM (
            SELECT bot_id, owner, type, name, short, invite, servers, shards, votes, certified, unnest(tags) AS tag_unnest FROM bots
        ) bots WHERE type = 'approved' AND (name ILIKE $1 OR owner ILIKE $1 OR short ILIKE $1 OR tag_unnest ILIKE $1) ORDER BY votes DESC, certified DESC LIMIT 6",
        "%".to_string() + &query + "%"
    )
    .fetch_all(pool)
    .await?;

    let mut search_bots = Vec::new();

    for bot in bots {
        search_bots.push(SearchBot {
            bot_id: bot.bot_id,
            name: bot.name,
            description: bot.short,
            invite: bot.invite,
            servers: bot.servers,
            shards: bot.shards,
            votes: bot.votes,
            certified: bot.certified,
        });
    }

    let packs = sqlx::query!(
        "SELECT DISTINCT name, short, bots, votes FROM (
            SELECT name, short, owner, bots, votes, unnest(bots) AS bot_unnest FROM packs
        ) packs WHERE (name ILIKE $1 OR bot_unnest ILIKE $1 OR short ILIKE $1 OR owner ILIKE $1) LIMIT 6",
        "%".to_string() + &query + "%"
    )
    .fetch_all(pool)
    .await?;

    let mut search_packs = Vec::new();

    for pack in packs {
        search_packs.push(SearchPack {
            name: pack.name,
            description: pack.short,
            bots: pack.bots,
            votes: pack.votes
        });
    }

    let users = sqlx::query!(
        "SELECT DISTINCT users.user_id, users.about FROM users 
        INNER JOIN bots ON bots.owner = users.user_id 
        WHERE (bots.name ilike $1 OR bots.short ilike $1 OR bots.bot_id ilike $1) 
        OR (users.username ilike $1) LIMIT 12",
        "%".to_string() + &query + "%"
    )
    .fetch_all(pool)
    .await?;

    let mut search_users = Vec::new();

    for user in users {
        search_users.push(SearchUser {
            user_id: user.user_id,
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
use std::sync::Arc;

use crate::types::{Error, Search, SearchBot, SearchPack, SearchUser};

use crate::public::{get_user, AvacadoPublic};

use sqlx::PgPool;

pub async fn search_bots(
    query: &String,
    pool: &PgPool,
    public: &AvacadoPublic,
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
            votes: pack.votes,
        });

        for bot in pack.bots {
            let res = sqlx::query!(
                "SELECT bot_id, name, short, invite, servers, shards, votes, certified, tags FROM bots WHERE bot_id = $1",
                bot
            )
            .fetch_one(pool)
            .await;

            if res.is_err() {
                continue;
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
            about: user.about,
        });
    }

    let res = Arc::new(Search {
        bots: search_bots,
        packs: search_packs,
        users: search_users,
    });

    public.search_cache.insert(query.clone(), res.clone()).await;

    Ok(res)
}

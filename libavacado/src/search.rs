use std::sync::Arc;

use crate::types::{Error, Search, SearchBot, SearchPack, SearchUser};

use crate::public::{get_user, AvacadoPublic};

use sqlx::PgPool;

pub struct SearchFilter {
    pub from: Option<i32>,
    pub to: Option<i32>,
}

impl SearchFilter {
    pub fn from(self: &Self) -> i32 {
        self.from.unwrap_or(-1)
    }

    pub fn to(self: &Self) -> i32 {
        self.to.unwrap_or(-1)
    }
}

pub struct SearchOpts {
    pub gc: SearchFilter,
    pub votes: SearchFilter,
    pub servers: SearchFilter,
}

impl SearchOpts {
    /// Returns the cache key
    pub fn key(self: &Self) -> String {
        return format!(
            ":{}-{}:{}-{}-{}-{}",
            self.gc.from(),
            self.gc.to(),
            self.votes.from(),
            self.votes.to(),
            self.servers.from(),
            self.servers.to()
        );
    }
}

/*
Core search concepts:

To add a filter:

AND (bots.FIELD >= $N) -- FROM
AND (($N+1 = -1) OR (bots.FIELD <= $N+1)) -- TO
*/

pub async fn search_bots(
    query: &String,
    pool: &PgPool,
    public: &AvacadoPublic,
    opts: &SearchOpts,
) -> Result<Arc<Search>, Error> {
    let search = public.search_cache.get(&(query.to_string() + &opts.key()));

    if search.is_some() {
        let search_inf = search.unwrap().clone();
        return Ok(search_inf.into());
    }

    let bots = sqlx::query!(
        "SELECT DISTINCT bot_id, clicks, invite_clicks, vanity, type, banner, name, short, invite, servers, shards, votes, certified, tags FROM (
            SELECT bot_id, clicks, invite_clicks, vanity, owner, name, type, banner, short, invite, servers, shards, votes, certified, tags, unnest(tags) AS tag_unnest FROM bots
        ) bots 
        WHERE type = 'approved' 
        AND (name ILIKE $2 OR owner @@ $1 OR short @@ $1 OR tag_unnest @@ $1) 

        -- Guild count filter (3-4)
        AND (servers >= $3)
        AND (($4 = -1) OR (servers <= $4))

        -- Votes filter (5-6)
        AND (votes >= $5)
        AND (($6 = -1) OR (votes <= $6))

        -- Servers filter (7-8)
        AND (servers >= $7)
        AND (($8 = -1) OR (servers <= $8))

        ORDER BY votes DESC, certified DESC LIMIT 6",
        query,
        "%".to_string() + query + "%",
        opts.gc.from(),
        opts.gc.to(),
        opts.votes.from(),
        opts.votes.to(),
        opts.servers.from(),
        opts.servers.to()
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
	    r#type: bot.r#type,
	    banner: bot.banner,
	    vanity: bot.vanity,
	    clicks: bot.clicks, 
	    invite_clicks: bot.invite_clicks
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
                "SELECT type, vanity, clicks, invite_clicks, banner, bot_id, name, short, invite, servers, shards, votes, certified, tags FROM bots WHERE bot_id = $1",
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
		r#type: res.r#type,
		banner: res.banner,
		vanity: res.vanity,
		clicks: res.clicks,
		invite_clicks: res.invite_clicks
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

    public.search_cache.insert(query.to_string() + &opts.key(), res.clone()).await;

    Ok(res)
}

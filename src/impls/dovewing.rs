use serde::{Deserialize, Serialize};
use serenity::all::UserId;
use sqlx::PgPool;
use ts_rs::TS;
use utoipa::ToSchema;

use super::cache::CacheHttpImpl;

#[derive(Clone, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = ".generated/PlatformUser.ts")]
pub struct PlatformUser {
    pub id: String,
    pub username: String,
    pub avatar: String,
    pub display_name: String,
    pub bot: bool,
    pub status: String,
}

impl PartialEq for PlatformUser {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone)]
pub enum DovewingSource {
    Discord(CacheHttpImpl)
}

impl DovewingSource {
    /// Returns the expiry time of a user
    pub fn user_expiry_time(&self) -> i64 {
        match self {
            // 8 hours
            DovewingSource::Discord(_) => 8 * 60 * 60,
        }
    }

    /// Returns a cached user if available
    pub fn cached_user(&self, user_id: &str) -> Result<Option<PlatformUser>, crate::Error> {
        match self {
            DovewingSource::Discord(c) => {
                let Ok(uid) = user_id.parse::<UserId>() else {
                    return Err("Invalid user id".into());
                };

                for gid in c.cache.guilds() {
                    if let Some(member) = c.cache.member(gid, uid) {
                        // Check precenses for status
                        let p = {
                            let guild = c.cache.guild(gid);

                            if let Some(guild) = guild {
                                let p = guild.presences.get(&uid);
                                p.cloned()
                            } else {
                                None
                            }
                        };
                        
                        return Ok(Some(PlatformUser {
                            id: user_id.to_string(),
                            username: member.user.name.clone().to_string(),
                            display_name: {
                                if let Some(ref display_name) = member.user.global_name {
                                    display_name.clone()
                                } else {
                                    member.user.name.clone()
                                }
                            }.to_string(),
                            bot: member.user.bot,
                            avatar: member.user.face(),
                            status: if let Some(p) = p {
                                match p.status {
                                    serenity::model::user::OnlineStatus::Online => "online",
                                    serenity::model::user::OnlineStatus::Idle => "idle",
                                    serenity::model::user::OnlineStatus::DoNotDisturb => "dnd",
                                    serenity::model::user::OnlineStatus::Invisible => "invisible",
                                    serenity::model::user::OnlineStatus::Offline => "offline",
                                    _ => "offline",
                                }.to_string()
                            } else {
                                "offline".to_string()
                            },
                        }));
                    }
                }
                
                Ok(None)
            },
        }
    }

    pub async fn http_user(&self, user_id: &str) -> Result<PlatformUser, crate::Error> {
        match self {
            DovewingSource::Discord(c) => {

                let Ok(uid) = user_id.parse::<UserId>() else {
                    return Err("Invalid user id".into());
                };

                let user = uid.to_user(&c.http).await?;

                Ok(PlatformUser {
                    id: user_id.to_string(),
                    username: user.name.clone().to_string(),
                    display_name: {
                        if let Some(ref display_name) = user.global_name {
                            display_name.clone()
                        } else {
                            user.name.clone()
                        }
                    }.to_string(),
                    bot: user.bot,
                    avatar: user.face(),
                    status: "offline".to_string(),
                })
            }
        }
    }
}

pub async fn get_platform_user(pool: &PgPool, src: DovewingSource, user_id: &str) -> Result<PlatformUser, crate::Error> {
    // First check cache_http
    let cached_uid = src.cached_user(user_id)?;

    if let Some(cached_uid) = cached_uid {
        // Update internal_user_cache__discord
        sqlx::query!(
            "INSERT INTO internal_user_cache__discord (id, username, display_name, avatar, bot) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET username = $2, display_name = $3, avatar = $4, bot = $5",
            user_id,
            cached_uid.username,
            cached_uid.display_name,
            cached_uid.avatar,
            cached_uid.bot,
        )
        .execute(pool)
        .await?;

        return Ok(cached_uid);
    }

    // Then check internal_user_cache__discord
    let rec = sqlx::query!(
        "SELECT username, display_name, avatar, bot, last_updated FROM internal_user_cache__discord WHERE id = $1",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(rec) = rec {
        if rec.last_updated.timestamp() + src.user_expiry_time() < chrono::Utc::now().timestamp() {
            // Make a tokio task to update the cache
            let pool = pool.clone();
            let src = src.clone();
            let user_id = user_id.to_string();

            tokio::spawn(async move {
                let user = src.http_user(&user_id).await?;

                sqlx::query!(
                    "UPDATE internal_user_cache__discord SET username = $1, display_name = $2, avatar = $3, bot = $4, last_updated = NOW() WHERE id = $5",
                    user.username,
                    user.display_name,
                    user.avatar,
                    user.bot,
                    user_id,
                )
                .execute(&pool)
                .await?;

                Ok::<(), crate::Error>(())
            });
        }

        Ok(PlatformUser {
            id: user_id.to_string(),
            username: rec.username,
            display_name: rec.display_name,
            bot: rec.bot,
            avatar: rec.avatar,
            status: "offline".to_string(),
        })
    } else {
        // Fetch from http
        let user = src.http_user(user_id).await?;

        sqlx::query!(
            "INSERT INTO internal_user_cache__discord (id, username, display_name, avatar, bot) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET username = $2, display_name = $3, avatar = $4, bot = $5",
            user_id,
            user.username,
            user.display_name,
            user.avatar,
            user.bot,
        )
        .execute(pool)
        .await?;

        Ok(user)
    }
}

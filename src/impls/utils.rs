use sqlx::PgPool;

use super::target_types::TargetType;

pub struct EntityManagers {
    users: Vec<Manager>,
}

struct Manager {
    mentionable: bool,
    user: String,
}

impl EntityManagers {
    pub fn all(&self) -> Vec<String> {
        let mut all = Vec::new();

        for manager in &self.users {
            all.push(manager.user.clone());
        }

        all
    }

    #[allow(dead_code)]
    pub fn mentionables(&self) -> Vec<String> {
        let mut mentionable = Vec::new();

        for manager in &self.users {
            if manager.mentionable {
                mentionable.push(manager.user.clone());
            }
        }

        mentionable
    }

    pub fn mention_users(&self) -> String {
        let mut mentionable = Vec::new();

        for manager in &self.users {
            if manager.mentionable {
                mentionable.push("<@".to_string() + &manager.user + ">");
            }
        }

        mentionable.join(", ")
    }
}

pub async fn get_entity_managers(
    target_type: TargetType,
    target_id: &str,
    pool: &PgPool,
) -> Result<EntityManagers, crate::Error> {
    let team_id = match target_type {
        TargetType::Bot => {
            // Check for owner first
            let owner_rec = sqlx::query!("SELECT owner FROM bots WHERE bot_id = $1", target_id)
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    format!("Error while checking for owner of bot {}: {}", target_id, e)
                })?;

            if let Some(owner) = owner_rec.owner {
                return Ok(EntityManagers {
                    users: vec![Manager {
                        mentionable: true,
                        user: owner,
                    }],
                });
            } else {
                let team_id =
                    sqlx::query!("SELECT team_owner FROM bots WHERE bot_id = $1", target_id)
                        .fetch_one(pool)
                        .await
                        .map_err(|e| {
                            format!(
                                "Error while checking for team owner of bot {}: {}",
                                target_id, e
                            )
                        })?;

                if let Some(team_id) = team_id.team_owner {
                    // Get all team members first
                    team_id
                } else {
                    return Err(format!(
                        "Bot {} is not owned by a team or a user. Please contact a dev right now!",
                        target_id
                    )
                    .into());
                }
            }
        }
        TargetType::Server => {
            let team_owner = sqlx::query!(
                "SELECT team_owner FROM servers WHERE server_id = $1",
                target_id
            )
            .fetch_one(pool)
            .await
            .map_err(|e| {
                format!(
                    "Error while checking for team owner of server {}: {}",
                    target_id, e
                )
            })?;

            team_owner.team_owner
        }
        TargetType::Team => sqlx::types::Uuid::parse_str(target_id)
            .map_err(|e| format!("Error while parsing team id {}: {}", target_id, e))?,
        TargetType::Pack => {
            return Err("Packs are not supported yet!".into());
        }
    };

    let team_members = sqlx::query!(
        "SELECT user_id, mentionable FROM team_members WHERE team_id = $1",
        team_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        format!(
            "Error while getting team members of team {}: {}",
            team_id, e
        )
    })?;

    if team_members.is_empty() {
        return Err(format!(
            "Entity {} is on a team with no members. Please contact a dev right now!",
            target_id
        )
        .into());
    }

    // Return all members
    Ok(EntityManagers {
        users: team_members
            .iter()
            .map(|m| Manager {
                mentionable: m.mentionable,
                user: m.user_id.clone(),
            })
            .collect(),
    })
}

pub struct OwnedBy {
    pub target_type: String,
    pub target_id: String,
    pub entity_state: String,
}

pub async fn get_owned_by(user_id: &str, pool: &PgPool) -> Result<Vec<OwnedBy>, crate::Error> {
    // Check for directly owned first
    let owned = sqlx::query!("SELECT bot_id, type FROM bots WHERE owner = $1", user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            format!(
                "Error while checking for owned bots of user {}: {}",
                user_id, e
            )
        })?;

    let mut owned_by = Vec::new();

    for bot in owned {
        owned_by.push(OwnedBy {
            target_type: "bot".to_string(),
            target_id: bot.bot_id,
            entity_state: bot.r#type,
        });
    }

    // Check for team owned
    let user_teams = sqlx::query!(
        "SELECT team_id FROM team_members WHERE user_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error while checking for teams of user {}: {}", user_id, e))?;

    for team in user_teams {
        let team_bots = sqlx::query!(
            "SELECT bot_id, type FROM bots WHERE team_owner = $1",
            team.team_id
        )
        .fetch_all(pool)
        .await
        .map_err(|e| {
            format!(
                "Error while checking for team owned bots of team {}: {}",
                team.team_id, e
            )
        })?;

        for bot in team_bots {
            owned_by.push(OwnedBy {
                target_type: "bot".to_string(),
                target_id: bot.bot_id,
                entity_state: bot.r#type,
            });
        }
    }

    Ok(owned_by)
}

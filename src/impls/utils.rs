use kittycat::perms::{PartialStaffPosition, Permission, StaffPermissions};

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
        TargetType::User => {
            let user = sqlx::query!("SELECT user_id FROM users WHERE user_id = $1", target_id)
                .fetch_one(pool)
                .await;

            match user {
                Ok(record) => {
                    return Ok(EntityManagers {
                        users: vec![Manager {
                            mentionable: true,
                            user: record.user_id,
                        }],
                    });
                }
                Err(sqlx::Error::RowNotFound) => {
                    return Err(format!("User {} not found.", target_id).into())
                }
                Err(e) => {
                    return Err(
                        format!("Error while checking for user {}: {}", target_id, e).into(),
                    )
                }
            }
        }
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

#[allow(dead_code)]
pub struct OwnedBy {
    pub target_type: TargetType,
    pub target_id: String,
    pub entity_state: String,
}

pub async fn get_owned_by(user_id: &str, pool: &PgPool) -> Result<Vec<OwnedBy>, crate::Error> {
    let query = sqlx::query!(
        r#"
        SELECT bot_id as id, type, 'bot' as entity
        FROM bots
        WHERE team_owner IN (SELECT team_id FROM team_members WHERE user_id = $1)

        UNION

        SELECT server_id as id, type, 'server' as entity
        FROM servers
        WHERE team_owner IN (SELECT team_id FROM team_members WHERE user_id = $1)

        UNION

        SELECT url as id, 'pack' as type, 'pack' as entity
        FROM packs
        WHERE owner = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error while executing query for user {}: {}", user_id, e))?;

    let mut owned_by = Vec::new();

    for row in query {
        let target = match row.entity.unwrap().as_str() {
            "bot" => Ok(TargetType::Bot),
            "server" => Ok(TargetType::Server),
            "pack" => Ok(TargetType::Pack),
            _ => Err("Unknown entity type encountered"),
        };

        let entry = match target {
            Ok(target_type) => OwnedBy {
                target_type,
                target_id: row.id.unwrap(),
                entity_state: row.r#type.unwrap(),
            },
            Err(err) => {
                eprintln!("Error: {}", err);
                continue;
            }
        };

        owned_by.push(entry);
    }

    Ok(owned_by)
}

/// Get the permissions of a user
pub async fn get_user_perms(
    pool: &PgPool,
    user_id: &str,
) -> Result<StaffPermissions, crate::Error> {
    let rec = sqlx::query!(
        "SELECT positions, perm_overrides FROM staff_members WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Error while getting staff perms of user {}: {}", user_id, e))?;

    let pos = sqlx::query!(
        "SELECT id, index, perms FROM staff_positions WHERE id = ANY($1)",
        &rec.positions
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error while getting staff perms of user {}: {}", user_id, e))?;

    Ok(StaffPermissions {
        user_positions: pos
            .iter()
            .map(|p| PartialStaffPosition {
                id: p.id.hyphenated().to_string(),
                index: p.index,
                perms: p
                    .perms
                    .iter()
                    .map(|x| Permission::from_string(x))
                    .collect::<Vec<Permission>>(),
            })
            .collect(),
        perm_overrides: rec
            .perm_overrides
            .iter()
            .map(|x| Permission::from_string(x))
            .collect::<Vec<Permission>>(),
    })
}

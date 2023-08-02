use sqlx::PgPool;

/// DEPRECATED
/// TODO: Fix this function with Mentionable once added
pub async fn resolve_ping_user(bot_id: &str, pool: &PgPool) -> Result<String, crate::Error> {
    // Check for owner first
    let owner_rec = sqlx::query!("SELECT owner FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Error while checking for owner of bot {}: {}", bot_id, e))?;

    if let Some(owner) = owner_rec.owner {
        Ok(owner)
    } else {
        let team_id = sqlx::query!("SELECT team_owner FROM bots WHERE bot_id = $1", bot_id)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                format!(
                    "Error while checking for team owner of bot {}: {}",
                    bot_id, e
                )
            })?;

        if let Some(team_id) = team_id.team_owner {
            // Get all team members first

            let team_members = sqlx::query!(
                "SELECT user_id, flags FROM team_members WHERE team_id = $1",
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

            let mut owner = None;

            // Try to find owner
            for member in &team_members {
                if member.flags.contains(&"global.*".to_string()) {
                    owner = Some(member.user_id.clone());
                    break;
                }
            }

            if let Some(owner) = owner {
                Ok(owner)
            } else if !team_members.is_empty() {
                Ok(team_members[0].user_id.clone())
            } else {
                Err(format!(
                    "Bot {} is on a team no owner or team members. Please contact a dev right now!",
                    bot_id
                )
                .into())
            }
        } else {
            Err(format!(
                "Bot {} has no owner or team owner. Please contact a dev right now!",
                bot_id
            )
            .into())
        }
    }
}

pub struct OwnedBy {
    pub id: String,
    pub bot_type: String,
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
            id: bot.bot_id,
            bot_type: bot.r#type,
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
                id: bot.bot_id,
                bot_type: bot.r#type,
            });
        }
    }

    Ok(owned_by)
}

#[allow(dead_code)]
pub async fn get_bot_members(bot_id: &str, pool: &PgPool) -> Result<Vec<String>, crate::Error> {
    // Check for owner first
    let owner_rec = sqlx::query!("SELECT owner FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Error while checking for owner of bot {}: {}", bot_id, e))?;

    if let Some(owner) = owner_rec.owner {
        Ok(vec![owner])
    } else {
        let team_id = sqlx::query!("SELECT team_owner FROM bots WHERE bot_id = $1", bot_id)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                format!(
                    "Error while checking for team owner of bot {}: {}",
                    bot_id, e
                )
            })?;

        if let Some(team_id) = team_id.team_owner {
            let team_members = sqlx::query!(
                "SELECT user_id FROM team_members WHERE team_id = $1",
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

            let mut members = Vec::new();

            for member in &team_members {
                members.push(member.user_id.clone());
            }

            Ok(members)
        } else {
            Err(format!(
                "Bot {} has no owner or team owner. Please contact a dev right now!",
                bot_id
            )
            .into())
        }
    }
}

use log::{info, warn};

pub async fn team_cleaner(ctx: &serenity::all::Context) -> Result<(), crate::Error> {
    let data = ctx.data::<crate::Data>();
    let pool = &data.pool;

    // Create a transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Error creating transaction: {:?}", e))?;

    let res = sqlx::query!("SELECT id FROM teams")
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Error while fetching all teams: {}", e))?;

    info!("Found {} teams totally", res.len());

    for rec in res {
        let team_id = rec.id;

        // Check if team has members
        if sqlx::query!(
            "SELECT COUNT(*) FROM team_members WHERE team_id = $1",
            team_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            format!(
                "Error while checking if team {} has members: {}",
                team_id, e
            )
        })?
        .count
        .unwrap_or(0)
            == 0
        {
            // Delete team
            sqlx::query!("DELETE FROM teams WHERE id = $1", team_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Error while deleting team {}: {}", team_id, e))?;

            info!("Deleted team {}", team_id);
            continue;
        }

        // Ensure team_members perm array has Global Owner in it
        let tm_with_global_owner = sqlx::query!(
            "SELECT user_id FROM team_members WHERE team_id = $1 AND flags @> ARRAY['global.*']",
            team_id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            format!(
                "Error while checking count of team_members with global owner: {}: {}",
                team_id, e
            )
        })?;

        if tm_with_global_owner.is_empty() {
            let (user_id, _has_dh) = {
                let dh = sqlx::query!(
                    "SELECT user_id FROM team_members WHERE team_id = $1 AND data_holder = true",
                    team_id
                )
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| {
                    format!(
                        "Error while fetching data_holder for team {}: {}",
                        team_id, e
                    )
                })?;

                if let Some(dh) = dh {
                    (dh.user_id, true)
                } else {
                    let res = sqlx::query!(
                        "SELECT user_id FROM team_members WHERE team_id = $1 LIMIT 1",
                        team_id
                    )
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| {
                        format!(
                            "Error while fetching first team member for team {}: {}",
                            team_id, e
                        )
                    })?;

                    (res.user_id, false)
                }
            };

            sqlx::query!(
                "UPDATE team_members SET flags = $1, data_holder = $2 WHERE team_id = $3 AND user_id = $4",
                &["global.*".to_string()],
                true,
                team_id,
                user_id
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error while updating flags for team {}: {}", team_id, e))?;
        }

        // Ensure the team has at least one data_holder
        if sqlx::query!(
            "SELECT COUNT(*) FROM team_members WHERE team_id = $1 AND data_holder = true",
            team_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Error while validating data holders of {}: {}", team_id, e))?
        .count
        .unwrap_or(0)
            == 0
        {
            // Set a team member whose flags contains global.* to data_holder
            if !tm_with_global_owner.is_empty() {
                sqlx::query!(
                    "UPDATE team_members SET data_holder = true WHERE team_id = $1 AND user_id = $2",
                    team_id,
                    tm_with_global_owner[0].user_id,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Error while updating data_holder for team {}: {}", team_id, e))?;
            } else {
                warn!("Team {} has no data holders and no global owners", team_id);
            }
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Error while committing transaction: {:?}", e))?;

    Ok(())
}

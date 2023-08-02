use log::info;

pub async fn team_cleaner(pool: &sqlx::PgPool) -> Result<(), crate::Error> {
    // Get all teams with no members
    let res = sqlx::query!("SELECT team_id FROM team_members")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error while fetching all teams: {}", e))?;

    info!("Found {} teams totally", res.len());

    for rec in res {
        let team_id = rec.team_id;

        // Check if team has members
        let res = sqlx::query!(
            "SELECT COUNT(*) FROM team_members WHERE team_id = $1",
            team_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| {
            format!(
                "Error while checking if team {} has members: {}",
                team_id, e
            )
        })?;

        let count = res.count.unwrap_or(0);

        if count == 1 {
            // Ensure team_members perm array has Global Owner in it
            let tm = sqlx::query!("SELECT flags FROM team_members WHERE team_id = $1", team_id,)
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    format!(
                        "Error while checking if team {} has members: {}",
                        team_id, e
                    )
                })?;

            let mut flags: Vec<String> = tm.flags;

            if !flags.contains(&"global.*".to_string()) {
                // Give them global owner, and add
                flags.push("global.*".to_string());

                sqlx::query!(
                    "UPDATE team_members SET flags = $1 WHERE team_id = $2",
                    &flags,
                    team_id,
                )
                .execute(pool)
                .await
                .map_err(|e| format!("Error while updating flags for team {}: {}", team_id, e))?;

                info!(
                    "Added global.* to flags for team {} due to having only 1 member WITHOUT a Global Owner",
                    team_id
                );
            }
        }

        if count > 0 {
            continue;
        }

        // Delete team
        sqlx::query!("DELETE FROM teams WHERE id = $1", team_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Error while deleting team {}: {}", team_id, e))?;

        info!("Deleted team {}", team_id);
    }

    Ok(())
}

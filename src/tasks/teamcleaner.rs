use log::info;

pub async fn team_cleaner(
    pool: &sqlx::PgPool,
) -> Result<(), crate::Error> {
    // Get all teams with no members
    let res = sqlx::query!(
        "SELECT id FROM teams WHERE id NOT IN (
            SELECT team_id FROM team_members
        )"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error while checking for empty teams: {}", e))?;

    info!("Found {} empty teams", res.len());

    for rec in res {
        let team_id = rec.id;

        // Delete team
        sqlx::query!("DELETE FROM teams WHERE id = $1", team_id)
            .execute(pool)
            .await
            .map_err(|e| {
                format!(
                    "Error while deleting team {}: {}",
                    team_id, e
                )
            })?;

        info!("Deleted team {}", team_id);
    }

    Ok(())
}
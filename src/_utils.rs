pub async fn add_action_log(
    pool: &sqlx::PgPool,
    bot_id: String, 
    staff_id: String,
    reason: String,
    event_type: String
) -> Result<(), crate::Error> {
    sqlx::query!(
        "INSERT INTO action_logs (bot_id, staff_id, action_reason, event) VALUES ($1, $2, $3, $4)",
        bot_id,
        staff_id,
        reason,
        event_type
    )
    .execute(pool)
    .await?;
    Ok(())
}
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};

pub async fn vote_resetter(ctx: &serenity::client::Context) -> Result<(), crate::Error> {
    let data = ctx.data::<crate::Data>();
    let pool = &data.pool;

    let mut tx = pool.begin().await?;

    // Check that the last automated vote was 1 month ago
    let last_vote = sqlx::query!(
        "SELECT id FROM automated_vote_resets WHERE created_at > NOW() - INTERVAL '1 month' FOR UPDATE"
    )
    .fetch_optional(&mut *tx)
    .await?;

    if last_vote.is_some() {
        return Ok(());
    }

    // Acquire lock on entity_votes
    let _lock = sqlx::query!("LOCK TABLE entity_votes IN EXCLUSIVE MODE")
        .fetch_optional(&mut *tx)
        .await?;

    // Set voided to true
    sqlx::query!(
        "UPDATE entity_votes SET void = TRUE, void_reason = 'Automated votes reset', voided_at = NOW() WHERE void = false AND immutable = false"
    )
    .execute(&mut *tx)
    .await?;

    // Insert into automated_vote_resets
    sqlx::query!("INSERT INTO automated_vote_resets (created_at) VALUES (NOW())")
        .execute(&mut *tx)
        .await?;

    // Commit
    tx.commit().await?;

    // Send message to #bot-logs
    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("__Automated Per-Monthly Vote Reset!__")
            .footer(CreateEmbedFooter::new("Welcome back :)"))
            .color(0xFF0000),
    );

    crate::config::CONFIG
        .channels
        .mod_logs
        .send_message(&ctx.http, msg)
        .await?;

    Ok(())
}

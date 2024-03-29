use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};

const ENTITY_TYPES: [&str; 4] = ["bots", "servers", "teams", "packs"];

pub async fn vote_resetter(
    pool: &sqlx::PgPool,
    cache_http: &crate::impls::cache::CacheHttpImpl,
) -> Result<(), crate::Error> {
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
        "UPDATE entity_votes SET void = TRUE, void_reason = 'Automated votes reset', voided_at = NOW() WHERE void = false"
    )
    .execute(&mut *tx)
    .await?;

    // Clear entity-specific tables
    for entity_type in ENTITY_TYPES.iter() {
        sqlx::query(&format!("UPDATE {} SET votes = 0", entity_type))
            .execute(&mut *tx)
            .await?;
    }

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
        .send_message(cache_http, msg)
        .await?;

    Ok(())
}

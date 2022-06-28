use poise::serenity_prelude::GuildId;
use rand::{distributions::Alphanumeric, Rng}; // 0.8

pub async fn add_action_log(
    pool: &sqlx::PgPool,
    bot_id: String,
    staff_id: String,
    reason: String,
    event_type: String,
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

pub async fn bot_owner_in_server(
    ctx: &crate::Context<'_>,
    bot_id: &str,
) -> Result<bool, crate::Error> {
    let data = ctx.data();
    let discord = ctx.discord();

    // Get owners and additional owners
    let owners = sqlx::query!(
        "SELECT owner, additional_owners FROM bots WHERE bot_id = $1",
        bot_id
    )
    .fetch_one(&data.pool)
    .await?;

    // Check if owner is in server ``MAIN_SERVER``
    let main_server = GuildId(std::env::var("MAIN_SERVER")?.parse::<u64>()?);

    let main_owner = owners.owner.parse::<u64>()?;

    let owner_in_server = discord
        .cache
        .member_field(main_server, main_owner, |f| f.user.id);

    if owner_in_server.is_some() {
        return Ok(true);
    }

    // Check additional owners
    for owner in owners.additional_owners {
        let owner = owner.parse::<u64>();

        if owner.is_err() {
            continue;
        }

        let owner = owner.unwrap();

        let owner_in_server = discord
            .cache
            .member_field(main_server, owner, |f| f.user.id);

        if owner_in_server.is_some() {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn gen_random(length: usize) -> String {
    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();

    s
}

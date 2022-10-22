type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

#[poise::command(
    category = "Bot Owner",
    prefix_command,
    slash_command,
    user_cooldown = 1
)]
pub async fn setstats(
    ctx: Context<'_>,
    #[description = "Bot ID to update"] bot_id: String,
    #[description = "The new guild count"] servers: Option<i32>,
    #[description = "The new shard count"] shards: Option<i32>,
    #[description = "The new user count"] users: Option<i32>,
) -> Result<(), Error> {
    let data = ctx.data();

    let owner = sqlx::query!("SELECT owner FROM bots WHERE bot_id = $1", bot_id)
        .fetch_one(&data.pool)
        .await?;

    if owner.owner != ctx.author().id.to_string() {
        return Err("You are not the owner of this bot".into());
    }

    if let Some(gc) = servers {
        sqlx::query!("UPDATE bots SET servers = $1 WHERE bot_id = $2", gc, bot_id)
            .execute(&data.pool)
            .await?;
    }

    if let Some(sc) = shards {
        sqlx::query!("UPDATE bots SET shards = $1 WHERE bot_id = $2", sc, bot_id)
            .execute(&data.pool)
            .await?;
    }

    if let Some(uc) = users {
        sqlx::query!("UPDATE bots SET users = $1 WHERE bot_id = $2", uc, bot_id)
            .execute(&data.pool)
            .await?;
    }

    ctx.say("Updated stats of bot!").await?;

    Ok(())
}

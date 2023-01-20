
use crate::_checks as checks;
use poise::serenity_prelude::RoleId;

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

/// Get your roles based on the bots you own
#[poise::command(
    category = "Bot Owner",
    prefix_command,
    slash_command,
    user_cooldown = 1,
    check = "checks::main_server"
)]
pub async fn getbotroles(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let data = ctx.data();

    let id = ctx.author().id.to_string();
    let id_vec = vec![id.clone()];
    let member = ctx.author_member().await;

    if member.is_none() {
        return Err("You are not in the server".into());
    }

    let mut member = member.unwrap().into_owned();

    let owner = sqlx::query!("SELECT bot_id, type FROM bots WHERE owner = $1 OR additional_owners && $2", id, &id_vec)
        .fetch_all(&data.pool)
        .await?;

    if owner.len() == 0 {
        return Err("You are not the owner/additional owner of any bots".into());
    }

    let mut approved = false;
    let mut certified = false;

    for bot in owner {
        if bot.r#type == "approved" {
            approved = true; // We need to check for certified as well
            continue;
        } 

        if bot.r#type == "certified" {
            approved = true;
            certified = true;
            break; // No need to check for more
        }
    }


    if !approved {
        return Err("You are not the owner/additional owner of any approved or certified bots".into());
    }

    if approved {
        ctx.say("You are the owner/additional owner of an approved bot! Giving you approved role").await?;
        let bot_role = std::env::var("BOT_DEV_ROLE").unwrap().parse::<RoleId>()?;
        
        member.add_role(&ctx, bot_role).await?;
    } else if certified {
        ctx.say("You are the owner/additional owner of a certified bot! Giving you certified role").await?;
        let bot_role = std::env::var("BOT_DEV_ROLE").unwrap().parse::<RoleId>()?;
        let certified_role = std::env::var("CERTIFIED_DEV_ROLE").unwrap().parse::<RoleId>()?;
        
        member.add_role(&ctx, bot_role).await?;
        member.add_role(&ctx, certified_role).await?;
    }

    Ok(())
}

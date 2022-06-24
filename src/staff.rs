use poise::serenity_prelude as serenity;

use std::fmt::Write as _; 
// import without risk of name clashing
use serenity::id::UserId;

use crate::checks;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Staff base command
#[poise::command(
    prefix_command,
    slash_command,
    guild_cooldown = 10,
    subcommands("staff_list")
)]
pub async fn staff(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Available options are ``staff list``").await?;
    Ok(())
}

#[poise::command(rename = "list", track_edits, prefix_command, slash_command, check = "checks::staff_server")]
pub async fn staff_list(ctx: Context<'_>) -> Result<(), Error> {
    // Get list of users with staff flag set to true
    let data = ctx.data();

    let staffs = sqlx::query!(
        "SELECT user_id, username FROM users WHERE staff = true ORDER BY user_id ASC"
    )
    .fetch_all(&data.pool)
    .await?;

    let mut staff_list = "**Staff List**\n".to_string();
    let mut not_in_staff_server = "**Not in staff server (based on cache, may be inaccurate)**\n".to_string();

    let guild = ctx.guild().unwrap();

    for staff in staffs.iter() {
        // Convert ID to u64
        let user_id = staff.user_id.parse::<u64>()?;

        let cache_user = ctx.discord().cache.member(guild.id, UserId(user_id));

        let user = match cache_user {
            Some(user) => user.user,
            None => {
                // User not found in cache, fetch from API
                let user = UserId(user_id).to_user(&ctx.discord().http).await?;

                write!(not_in_staff_server, "{} ({})", user.id.0, user.name)?;
                user
            }
        };

        writeln!(staff_list, "{} ({})", staff.user_id, user.name)?;
    }

    ctx.say(staff_list + "\n" + &not_in_staff_server).await?;

    Ok(())
}
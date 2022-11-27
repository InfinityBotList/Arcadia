use log::info;
use poise::serenity_prelude::{self as serenity, GuildId};

use std::{fmt::Write as _, num::NonZeroU64};
// import without risk of name clashing
use serenity::id::UserId;

use crate::{_checks as checks, _utils::Bool};

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Staff base command
#[poise::command(
    category = "Staff",
    prefix_command,
    slash_command,
    guild_cooldown = 10,
    subcommands(
        "staff_add",
        "staff_del",
        "staff_guildlist",
        "staff_guilddel",
        "staff_guildleave"
    )
)]
pub async fn staff(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Some available options are ``staff list``, ``staff guildlist`` (dev/admin only), ``staff_guilddel`` (dev/admin only), ``staff_guildleave`` (dev/admin only), ``staff recalc`` (dev/admin only), ``staff add`` (dev/admin only) etc.").await?;
    Ok(())
}

#[poise::command(
    rename = "list",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server"
)]
pub async fn staff_list(ctx: Context<'_>) -> Result<(), Error> {
    // Get list of users with staff flag set to true
    let data = ctx.data();
    let discord = &ctx.discord();

    sqlx::query!("UPDATE users SET user_id = TRIM(user_id)")
        .execute(&data.pool)
        .await?;

    // Remove user and warn
    let v = sqlx::query!("DELETE FROM users WHERE user_id = ''")
        .execute(&data.pool)
        .await?;

    if v.rows_affected() > 0 {
        info!("Removed {} users with empty user_id", v.rows_affected());
    }

    let staffs = sqlx::query!(
        "SELECT user_id, username, staff, admin, ibldev, iblhdev, hadmin FROM users WHERE (staff = true OR admin = true OR ibldev = true OR iblhdev = true OR hadmin = true) ORDER BY user_id ASC"
    )
    .fetch_all(&data.pool)
    .await?;

    let mut staff_list = "**Staff List**\n".to_string();
    let mut not_in_staff_server =
        "**Not in staff server (based on cache, may be inaccurate)**\n".to_string();

    let guild = ctx.guild().unwrap().id;

    for staff in staffs {
        // Convert ID to u64
        let user_id = staff.user_id.parse::<NonZeroU64>()?;

        let cache_user = discord.cache.member(guild, UserId(user_id));

        let user = match cache_user {
            Some(user) => user.user,
            None => {
                // User not found in cache, fetch from API
                let user = UserId(user_id).to_user(&ctx).await?;

                write!(not_in_staff_server, "{} ({})", user.id.0, user.name)?;
                user
            }
        };

        writeln!(
            staff_list,
            "{user_id} ({username}) [staff={staff}, admin={admin}, ibldev={ibldev}, iblhdev={iblhdev} hadmin={hadmin}]", 
            user_id=staff.user_id,
            username=user.name,
            staff=staff.staff,
            admin=staff.admin,
            ibldev=staff.ibldev,
            iblhdev=staff.iblhdev,
            hadmin=staff.hadmin,
        )?;
    }

    ctx.say(staff_list + "\n" + &not_in_staff_server).await?;

    Ok(())
}

/// Adds a new staff member
#[poise::command(
    rename = "add",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_admin_hdev"
)]
pub async fn staff_add(
    ctx: Context<'_>,
    #[description = "The user ID of the user to add"] member: serenity::Member,
    #[description = "Whether to give roles, true by default"] give_roles: Option<Bool>,
) -> Result<(), Error> {
    // Check if awaiting staff role in main server
    let main_server = std::env::var("MAIN_SERVER")
        .unwrap()
        .parse::<NonZeroU64>()
        .unwrap();

    let member = ctx.discord().cache.member(main_server, member.user.id);

    if member.is_none() {
        info!("Member not found in main server");
        return Err("User are not in the main server".into());
    }

    let mut member = member.unwrap();

    if give_roles.is_none() || give_roles.unwrap().to_bool() {
        let web_mod_role =
            poise::serenity_prelude::RoleId(std::env::var("WEB_MOD_ROLE")?.parse::<NonZeroU64>()?);

        if !member.roles.contains(&web_mod_role) {
            // Give user web mod role
            member.add_role(ctx.discord(), web_mod_role).await?;
        }
    }

    sqlx::query!(
        "UPDATE users SET staff = true WHERE user_id = $1",
        member.user.id.0.to_string()
    )
    .execute(&ctx.data().pool)
    .await?;

    ctx.say(&format!(
        "Added {} to the staff list (if they weren't already staff)",
        member.user.name
    ))
    .await?;

    Ok(())
}

/// Removes a staff member
#[poise::command(
    rename = "del",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::main_server",
    check = "checks::is_admin_hdev"
)]
pub async fn staff_del(
    ctx: Context<'_>,
    #[description = "The user ID of the user to remove staff from"] mut member: serenity::Member,
) -> Result<(), Error> {
    let staff_man_role =
        poise::serenity_prelude::RoleId(std::env::var("STAFF_MAN_ROLE")?.parse::<NonZeroU64>()?);
    let owner_role =
        poise::serenity_prelude::RoleId(std::env::var("OWNER_ROLE")?.parse::<NonZeroU64>()?);

    if member.user.id == ctx.author().id {
        // Don't error, just let them know how stupid they are
        ctx.say(
            "Removing yourselves from staff eh? Well I'll do it since you asked so nicely :heart:",
        )
        .await?;
    } else if member.roles.contains(&staff_man_role) {
        return Err(format!(
            "{} is a staff manager and as such is protected!",
            member.user.name
        )
        .into());
    } else if member.roles.contains(&owner_role) {
        return Err(format!("{} is a owner and as such is protected!", member.user.name).into());
    }

    sqlx::query!(
        "UPDATE users SET staff = false, ibldev = false, admin = false WHERE user_id = $1",
        member.user.id.0.to_string()
    )
    .execute(&ctx.data().pool)
    .await?;

    let web_mod_role =
        poise::serenity_prelude::RoleId(std::env::var("WEB_MOD_ROLE")?.parse::<NonZeroU64>()?);

    if member.roles.contains(&web_mod_role) {
        // Remove users web mod role
        member.remove_role(ctx.discord(), web_mod_role).await?;
    }

    let staff_server =
        poise::serenity_prelude::GuildId(std::env::var("MAIN_SERVER")?.parse::<NonZeroU64>()?);

    staff_server
        .kick_with_reason(
            &ctx.discord().http,
            member.user.id,
            "Removed from staff list",
        )
        .await?;

    ctx.say("Removed from staff list").await?;

    Ok(())
}

/// Get guild list
#[poise::command(
    rename = "guildlist",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_admin_hdev",
    check = "checks::staff_server"
)]
pub async fn staff_guildlist(ctx: Context<'_>) -> Result<(), Error> {
    let guilds = ctx.discord().cache.guilds();

    let mut guild_list = String::new();

    for guild in guilds.iter() {
        let name = guild
            .name(ctx.discord())
            .unwrap_or_else(|| "Unknown".to_string())
            + " ("
            + &guild.to_string()
            + ")\n";
        guild_list.push_str(&name);
    }

    ctx.say(&guild_list).await?;

    Ok(())
}

/// Delete server
#[poise::command(
    rename = "guilddel",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_admin_hdev",
    check = "checks::staff_server"
)]
pub async fn staff_guilddel(
    ctx: Context<'_>,
    #[description = "The guild ID to remove"] guild: String,
) -> Result<(), Error> {
    let gid = guild.parse::<NonZeroU64>()?;

    ctx.discord().http.delete_guild(GuildId(gid)).await?;

    ctx.say("Removed guild").await?;

    Ok(())
}

/// Delete server
#[poise::command(
    rename = "guildleave",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_admin_hdev",
    check = "checks::staff_server"
)]
pub async fn staff_guildleave(
    ctx: Context<'_>,
    #[description = "The guild ID to leave"] guild: String,
) -> Result<(), Error> {
    let gid = guild.parse::<NonZeroU64>()?;

    ctx.discord().http.leave_guild(GuildId(gid)).await?;

    ctx.say("Removed guild").await?;

    Ok(())
}

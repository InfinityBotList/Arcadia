use crate::config;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Check for main_server
pub async fn main_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_main_server = match ctx.guild_id() {
        Some(guild_id) => guild_id == config::CONFIG.servers.main,
        None => false,
    };

    Ok(in_main_server)
}

/// Check for staff_server
pub async fn staff_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_staff_server = match ctx.guild_id() {
        Some(guild_id) => guild_id == config::CONFIG.servers.staff,
        None => false,
    };

    Ok(in_staff_server)
}

/// Check for staff_server
pub async fn testing_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_testing_server = match ctx.guild_id() {
        Some(guild_id) => guild_id == config::CONFIG.servers.testing,
        None => false,
    };

    Ok(in_testing_server)
}

pub async fn is_staff(ctx: Context<'_>) -> Result<bool, Error> {
    let count = sqlx::query!(
        "SELECT COUNT(*) FROM staff_members WHERE user_id = $1",
        ctx.author().id.to_string()
    )
    .fetch_one(&ctx.data().pool)
    .await?
    .count
    .unwrap_or(0);

    if count == 0 {
        return Err("You are not staff".into());
    }

    Ok(true)
}

pub async fn needs_onboarding(ctx: Context<'_>) -> Result<bool, Error> {
    let staff = sqlx::query!(
        "SELECT staff_onboard_state FROM users WHERE user_id = $1",
        ctx.author().id.to_string()
    )
    .fetch_one(&ctx.data().pool)
    .await?;

    if staff.staff_onboard_state != "completed" {
        return Err("You need to complete onboarding to use this command!".into());
    }

    Ok(true)
}

pub async fn is_admin_hdev(ctx: Context<'_>) -> Result<bool, Error> {
    let staff = sqlx::query!(
        "SELECT admin, iblhdev FROM users WHERE user_id = $1",
        ctx.author().id.to_string()
    )
    .fetch_one(&ctx.data().pool)
    .await?;

    if !(staff.admin || staff.iblhdev) {
        return Err("You are not admin (manager) or a head developer".into());
    }

    Ok(true)
}

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Check for main_server
pub async fn main_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_main_server = match ctx.guild_id() {
        Some(guild_id) => guild_id.0 == libavacado::CONFIG.servers.main,
        None => false,
    };

    Ok(in_main_server)
}

/// Check for staff_server
pub async fn staff_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_staff_server = match ctx.guild_id() {
        Some(guild_id) => guild_id.0 == libavacado::CONFIG.servers.staff,
        None => false,
    };

    Ok(in_staff_server)
}

/// Check for staff_server
pub async fn testing_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_testing_server = match ctx.guild_id() {
        Some(guild_id) => guild_id.0 == libavacado::CONFIG.servers.testing,
        None => false,
    };

    Ok(in_testing_server)
}

pub async fn is_staff(ctx: Context<'_>) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT staff FROM users WHERE user_id = $1", ctx.author().id.to_string())
    .fetch_one(&ctx.data().pool)
    .await?;

    if !staff.staff {
        return Err("You are not staff".into());
    }

    Ok(true)
}

pub async fn is_admin_hdev(ctx: Context<'_>) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT admin, iblhdev FROM users WHERE user_id = $1", ctx.author().id.to_string())
        .fetch_one(&ctx.data().pool)
        .await?;

    if !(staff.admin || staff.iblhdev) {
        return Err("You are not admin (manager) or a head developer".into());
    }

    Ok(true)
}

pub async fn is_admin(ctx: Context<'_>) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT admin FROM users WHERE user_id = $1", ctx.author().id.to_string())
        .fetch_one(&ctx.data().pool)
        .await?;

    if !(staff.admin) {
        return Err("You are not admin (manager)".into());
    }

    Ok(true)
}

pub async fn is_hdev_hadmin(ctx: Context<'_>) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT hadmin, iblhdev FROM users WHERE user_id = $1", ctx.author().id.to_string())
    .fetch_one(&ctx.data().pool)
    .await?;

    if !(staff.hadmin || staff.iblhdev) {
        return Err("You are not hadmin (head manager) or a iblhdev (head developer)".into());
    }

    Ok(true)
}

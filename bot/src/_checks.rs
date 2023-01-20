type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

use libavacado::checks;

/// Check for main_server
pub async fn main_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_main_server = match ctx.guild_id() {
        Some(guild_id) => guild_id.0.to_string() == std::env::var("MAIN_SERVER")?,
        None => false,
    };

    Ok(in_main_server)
}


/// Check for staff_server
pub async fn staff_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_staff_server = match ctx.guild_id() {
        Some(guild_id) => guild_id.0.to_string() == std::env::var("STAFF_SERVER")?,
        None => false,
    };

    Ok(in_staff_server)
}

/// Check for staff_server
pub async fn testing_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_testing_server = match ctx.guild_id() {
        Some(guild_id) => guild_id.0.to_string() == std::env::var("TESTING_SERVER")?,
        None => false,
    };

    Ok(in_testing_server)
}

pub async fn is_staff(ctx: Context<'_>) -> Result<bool, Error> {
    checks::is_staff(&ctx.author().id.to_string(), &ctx.data().pool).await
}

pub async fn is_admin_hdev(ctx: Context<'_>) -> Result<bool, Error> {
    checks::is_admin_hdev(&ctx.author().id.to_string(), &ctx.data().pool).await
}

pub async fn is_any_staff(ctx: Context<'_>) -> Result<bool, Error> {
    checks::is_any_staff(&ctx.author().id.to_string(), &ctx.data().pool).await
}

pub async fn is_admin(ctx: Context<'_>) -> Result<bool, Error> {
    checks::is_admin(&ctx.author().id.to_string(), &ctx.data().pool).await
}

#[allow(dead_code)]
pub async fn is_hdev(ctx: Context<'_>) -> Result<bool, Error> {
    checks::is_hdev(&ctx.author().id.to_string(), &ctx.data().pool).await
}

pub async fn is_hdev_hadmin(ctx: Context<'_>) -> Result<bool, Error> {
    checks::is_hdev_hadmin(&ctx.author().id.to_string(), &ctx.data().pool).await
}

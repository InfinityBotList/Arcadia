type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

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

/// Check if user is staff
///
/// This check checks if the user has the `staff` bit set
pub async fn is_staff(ctx: Context<'_>) -> Result<bool, Error> {
    let data = ctx.data();

    let staff = sqlx::query!(
        "SELECT staff FROM users WHERE user_id = $1",
        ctx.author().id.0.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    Ok(staff.staff)
}

pub async fn is_admin_dev(ctx: Context<'_>) -> Result<bool, Error> {
    let data = ctx.data();

    let staff = sqlx::query!(
        "SELECT admin, ibldev FROM users WHERE user_id = $1",
        ctx.author().id.0.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    Ok(staff.admin || staff.ibldev)
}

pub async fn is_admin(ctx: Context<'_>) -> Result<bool, Error> {
    let data = ctx.data();

    let staff = sqlx::query!(
        "SELECT admin FROM users WHERE user_id = $1",
        ctx.author().id.0.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    Ok(staff.admin)
}

pub async fn is_hdev(ctx: Context<'_>) -> Result<bool, Error> {
    let data = ctx.data();

    let staff = sqlx::query!(
        "SELECT iblhdev FROM users WHERE user_id = $1",
        ctx.author().id.0.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    Ok(staff.iblhdev)
}

use sqlx::PgPool;

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Check if user is staff
///
/// This check checks if the user has the `staff` bit set
pub async fn is_staff(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!(
        "SELECT staff FROM users WHERE user_id = $1",
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(staff.staff)
}

pub async fn is_admin_hdev(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!(
        "SELECT admin, iblhdev FROM users WHERE user_id = $1",
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(staff.admin || staff.iblhdev)
}

pub async fn is_any_staff(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!(
        "SELECT staff, admin, ibldev, iblhdev FROM users WHERE user_id = $1",
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(staff.staff || staff.admin || staff.ibldev || staff.iblhdev)
}

pub async fn is_admin(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!(
        "SELECT admin FROM users WHERE user_id = $1",
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(staff.admin)
}

pub async fn is_hdev(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!(
        "SELECT iblhdev FROM users WHERE user_id = $1",
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(staff.iblhdev)
}

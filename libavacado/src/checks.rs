use sqlx::PgPool;

use crate::types::Error;

/// Check if user is staff
///
/// This check checks if the user has the `staff` bit set
pub async fn is_staff(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT staff FROM users WHERE user_id = $1", id)
        .fetch_one(pool)
        .await?;

    if !staff.staff {
        return Err("You are not staff".into());
    }

    Ok(true)
}

pub async fn is_admin_hdev(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT admin, iblhdev FROM users WHERE user_id = $1", id)
        .fetch_one(pool)
        .await?;

    if !(staff.admin || staff.iblhdev) {
        return Err("You are not admin (manager) or a head developer".into());
    }

    Ok(true)
}

pub async fn is_any_staff(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!(
        "SELECT staff, admin, ibldev, iblhdev FROM users WHERE user_id = $1",
        id
    )
    .fetch_one(pool)
    .await?;

    if !(staff.staff || staff.admin || staff.ibldev || staff.iblhdev) {
        return Err("You are not a staff of any form".into());
    }

    Ok(true)
}

pub async fn is_admin(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT admin FROM users WHERE user_id = $1", id)
        .fetch_one(pool)
        .await?;

    if !staff.admin {
        return Err("You are not admin".into());
    }

    Ok(true)
}

pub async fn is_hdev(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT iblhdev FROM users WHERE user_id = $1", id)
        .fetch_one(pool)
        .await?;

    if !staff.iblhdev {
        return Err("You are not a hdev (head developer)".into());
    }

    Ok(true)
}

pub async fn is_hdev_hadmin(id: &str, pool: &PgPool) -> Result<bool, Error> {
    let staff = sqlx::query!("SELECT iblhdev, hadmin FROM users WHERE user_id = $1", id)
        .fetch_one(pool)
        .await?;

    if !(staff.iblhdev || staff.hadmin) {
        return Err("You are not a hdev (head developer) or a hadmin (head staff manager)".into());
    }

    Ok(true)
}

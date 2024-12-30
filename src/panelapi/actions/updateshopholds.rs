use crate::impls::utils::get_user_perms;
use crate::panelapi::auth::check_auth;
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::shop_items::{ShopHold, ShopHoldAction};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use kittycat::perms;

pub async fn update_shop_holds(
    state: &AppState,
    login_token: String,
    action: ShopHoldAction,
) -> Result<Response, Error> {
    let auth_data = check_auth(&state.pool, &login_token)
        .await
        .map_err(Error::new)?;

    let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
        .await
        .map_err(Error::new)?
        .resolve();

    match action {
        ShopHoldAction::List => {
            let rows = sqlx::query!(
            "SELECT id, target_id, target_type, item, created_at, duration FROM shop_holds ORDER BY created_at ASC"
        )
        .fetch_all(&state.pool)
        .await
        .map_err(Error::new)?;

            let mut entries = Vec::new();

            for row in rows {
                entries.push(ShopHold {
                    id: row.id,
                    target_id: row.target_id,
                    target_type: row.target_type,
                    item: row.item,
                    created_at: row.created_at,
                    duration: row.duration.map(|d| {
                        let months = d.months as i64;
                        let days = d.days as i64;
                        let microseconds = d.microseconds;
                        let micros = months * 30 * 24 * 60 * 60 * 1_000_000
                            + days * 24 * 60 * 60 * 1_000_000
                            + microseconds;

                        micros / 1_000_000
                    }),
                });
            }

            Ok((StatusCode::OK, Json(entries)).into_response())
        }
        ShopHoldAction::Create {
            target_id,
            target_type,
            item,
            duration,
        } => {
            if !perms::has_perm(&user_perms, &"shop_holds.create".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to create shop holds [shop_holds.create]"
                        .to_string(),
                )
                    .into_response());
            }

            let item_exists = sqlx::query!("SELECT COUNT(*) FROM shop_items WHERE id = $1", item,)
                .fetch_one(&state.pool)
                .await
                .map_err(Error::new)?
                .count
                .unwrap_or(0)
                > 0;

            if !item_exists {
                return Ok(
                    (StatusCode::BAD_REQUEST, "Item does not exist".to_string()).into_response()
                );
            }

            if target_type != "bot" && target_type != "server" {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Target type must be either 'bot' or 'server'".to_string(),
                )
                    .into_response());
            }

            // Insert entry
            let duration = duration.map(|d| {
                // Make PgInterval from duration. Duration is in seconds
                let d = d as i64;
                sqlx::postgres::types::PgInterval {
                    microseconds: d * 1_000_000,
                    ..Default::default()
                }
            });

            let mut tx = state.pool.begin().await.map_err(Error::new)?;
            sqlx::query!(
                "INSERT INTO shop_holds (target_id, target_type, item, duration) VALUES ($1, $2, $3, $4)",
                target_id,
                target_type,
                item,
                duration,
            )
            .execute(&mut *tx)
            .await
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        ShopHoldAction::Edit {
            id,
            target_id,
            target_type,
            item,
            duration,
        } => {
            if !perms::has_perm(&user_perms, &"shop_holds.update".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to update shop holds [shop_holds.update]"
                        .to_string(),
                )
                    .into_response());
            }

            // Check if entry already exists with same id
            if sqlx::query!("SELECT COUNT(*) FROM shop_holds WHERE id = $1", id)
                .fetch_one(&state.pool)
                .await
                .map_err(Error::new)?
                .count
                .unwrap_or(0)
                == 0
            {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Entry with same id does not already exist".to_string(),
                )
                    .into_response());
            }

            let item_exists = sqlx::query!("SELECT COUNT(*) FROM shop_items WHERE id = $1", item,)
                .fetch_one(&state.pool)
                .await
                .map_err(Error::new)?
                .count
                .unwrap_or(0)
                > 0;

            if !item_exists {
                return Ok(
                    (StatusCode::BAD_REQUEST, "Item does not exist".to_string()).into_response()
                );
            }

            if target_type != "bot" && target_type != "server" {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Target type must be either 'bot' or 'server'".to_string(),
                )
                    .into_response());
            }

            // Update entry
            let duration = duration.map(|d| {
                // Make PgInterval from duration. Duration is in seconds
                let d = d as i64;
                sqlx::postgres::types::PgInterval {
                    microseconds: d * 1_000_000,
                    ..Default::default()
                }
            });

            sqlx::query!(
                "UPDATE shop_holds SET target_id = $1, target_type = $2, item = $3, duration = $4 WHERE id = $5",
                target_id,
                target_type,
                item,
                duration,
                id,
            )
            .execute(&state.pool)
            .await
            .map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        ShopHoldAction::Delete { id } => {
            if !perms::has_perm(&user_perms, &"shop_holds.delete".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to delete shop holds [shop_holds.delete]"
                        .to_string(),
                )
                    .into_response());
            }

            // Check if entry already exists
            if sqlx::query!("SELECT COUNT(*) FROM shop_holds WHERE id = $1", id)
                .fetch_one(&state.pool)
                .await
                .map_err(Error::new)?
                .count
                .unwrap_or(0)
                == 0
            {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Entry with same id does not already exist".to_string(),
                )
                    .into_response());
            }

            // Delete entry
            sqlx::query!("DELETE FROM shop_holds WHERE id = $1", id)
                .execute(&state.pool)
                .await
                .map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
    }
}

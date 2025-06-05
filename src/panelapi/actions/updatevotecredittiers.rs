use crate::impls::utils::get_user_perms;
use crate::panelapi::auth::check_auth;
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::vote_credit_tiers::{VoteCreditTier, VoteCreditTierAction};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use kittycat::perms;

pub async fn update_vote_credit_tiers(
    state: &AppState,
    login_token: String,
    action: VoteCreditTierAction,
) -> Result<Response, Error> {
    let auth_data = check_auth(&state.pool, &login_token)
        .await
        .map_err(Error::new)?;

    let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
        .await
        .map_err(Error::new)?
        .resolve();

    match action {
        VoteCreditTierAction::ListTiers => {
            let rows = sqlx::query!(
            "SELECT id, target_type, position, cents, votes, created_at FROM vote_credit_tiers ORDER BY position ASC"
        )
        .fetch_all(&state.pool)
        .await
        .map_err(Error::new)?;

            let mut entries = Vec::new();

            for row in rows {
                entries.push(VoteCreditTier {
                    id: row.id,
                    target_type: row.target_type,
                    position: row.position,
                    cents: row.cents,
                    votes: row.votes,
                    created_at: row.created_at,
                });
            }

            Ok((StatusCode::OK, Json(entries)).into_response())
        }
        VoteCreditTierAction::CreateTier {
            id,
            position,
            target_type,
            cents,
            votes,
        } => {
            if !perms::has_perm(&user_perms, &"vote_credit_tiers.create".into()) {
                return Ok((
                StatusCode::FORBIDDEN,
                "You do not have permission to create vote credit tiers [vote_credit_tiers.create]".to_string(),
            )
                .into_response());
            }

            if cents < 0.0 {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Cents cannot be lower than 0".to_string(),
                )
                    .into_response());
            }

            if votes < 0 {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Votes cannot be lower than 0".to_string(),
                )
                    .into_response());
            }

            if target_type != "bot" && target_type != "server" {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Target type must be either 'bot' or 'server'".to_string(),
                )
                    .into_response());
            }

            // Insert entry
            let mut tx = state.pool.begin().await.map_err(Error::new)?;
            sqlx::query!(
            "INSERT INTO vote_credit_tiers (id, target_type, position, cents, votes) VALUES ($1, $2, $3, $4, $5)",
            id,
            target_type,
            position,
            cents,
            votes,
        )
        .execute(&mut *tx)
        .await
        .map_err(Error::new)?;

            // Now keep shifting positions until they are all unique
            let mut index_a = position;

            loop {
                let rows = sqlx::query!(
                    "SELECT id, position FROM vote_credit_tiers WHERE position = $1 AND id != $2",
                    index_a,
                    id,
                )
                .fetch_all(&mut *tx)
                .await
                .map_err(Error::new)?;

                if rows.is_empty() {
                    break;
                }

                let mut index_b = index_a + 1;

                for row in rows {
                    sqlx::query!(
                        "UPDATE vote_credit_tiers SET position = $1 WHERE id = $2",
                        index_b,
                        row.id,
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    index_b += 1;
                }

                index_a = index_b;
            }

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        VoteCreditTierAction::EditTier {
            id,
            position,
            target_type,
            cents,
            votes,
        } => {
            if !perms::has_perm(&user_perms, &"vote_credit_tiers.update".into()) {
                return Ok((
                StatusCode::FORBIDDEN,
                "You do not have permission to update vote credit tiers [vote_credit_tiers.update]".to_string(),
            )
                .into_response());
            }

            // Check if entry already exists with same id
            if sqlx::query!("SELECT COUNT(*) FROM vote_credit_tiers WHERE id = $1", id)
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

            if cents < 0.0 {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Cents cannot be lower than 0".to_string(),
                )
                    .into_response());
            }

            if votes < 0 {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Votes cannot be lower than 0".to_string(),
                )
                    .into_response());
            }

            if target_type != "bot" && target_type != "server" {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Target type must be either 'bot' or 'server'".to_string(),
                )
                    .into_response());
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            // Update entry
            sqlx::query!(
            "UPDATE vote_credit_tiers SET position = $1, target_type = $2, cents = $3, votes = $4 WHERE id = $5",
            position,
            target_type,
            cents,
            votes,
            id,
        )
        .execute(&mut *tx)
        .await
        .map_err(Error::new)?;

            // Now keep shifting positions until they are all unique
            let mut index_a = position;

            loop {
                let rows = sqlx::query!(
                    "SELECT id, position FROM vote_credit_tiers WHERE position = $1 AND id != $2",
                    index_a,
                    id,
                )
                .fetch_all(&mut *tx)
                .await
                .map_err(Error::new)?;

                if rows.is_empty() {
                    break;
                }

                let mut index_b = index_a + 1;

                for row in rows {
                    sqlx::query!(
                        "UPDATE vote_credit_tiers SET position = $1 WHERE id = $2",
                        index_b,
                        row.id,
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(Error::new)?;

                    index_b += 1;
                }

                index_a = index_b;
            }

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        VoteCreditTierAction::DeleteTier { id } => {
            if !perms::has_perm(&user_perms, &"vote_credit_tiers.delete".into()) {
                return Ok((
                StatusCode::FORBIDDEN,
                "You do not have permission to delete vote credit tiers [vote_credit_tiers.delete]".to_string(),
            )
                .into_response());
            }

            // Check if entry already exists with same vesion
            if sqlx::query!("SELECT COUNT(*) FROM vote_credit_tiers WHERE id = $1", id)
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
            sqlx::query!("DELETE FROM vote_credit_tiers WHERE id = $1", id)
                .execute(&state.pool)
                .await
                .map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
    }
}

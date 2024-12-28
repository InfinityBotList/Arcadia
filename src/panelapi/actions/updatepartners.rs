use crate::impls::utils::get_user_perms;
use crate::panelapi::auth::check_auth;
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::partners::{
    CreatePartner, Partner, PartnerAction, PartnerType, Partners,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use kittycat::perms;
use sqlx::PgPool;

pub async fn update_partners(
    state: &AppState,
    login_token: String,
    action: PartnerAction,
) -> Result<Response, Error> {
    let auth_data = check_auth(&state.pool, &login_token)
        .await
        .map_err(Error::new)?;

    let user_perms = get_user_perms(&state.pool, &auth_data.user_id)
        .await
        .map_err(Error::new)?
        .resolve();

    async fn parse_partner(pool: &PgPool, partner: &CreatePartner) -> Result<(), crate::Error> {
        // Check if partner type exists
        let partner_type_exists =
            sqlx::query!("SELECT id FROM partner_types WHERE id = $1", partner.r#type)
                .fetch_optional(pool)
                .await?
                .is_some();

        if !partner_type_exists {
            return Err("Partner type does not exist".into());
        }

        // Ensure that image has been uploaded to CDN
        // Get cdn path from cdn_scope hashmap
        let cdn_scopes = crate::config::CONFIG.panel.cdn_scopes.get();

        let Some(cdn_path) = cdn_scopes.get(&crate::config::CONFIG.panel.main_scope) else {
            return Err("Main scope not found".into());
        };

        let path = format!("{}/avatars/partners/{}.webp", cdn_path.path, partner.id);

        match std::fs::metadata(&path) {
            Ok(m) => {
                if !m.is_file() {
                    return Err("Image does not exist".into());
                }

                if m.len() > 100_000_000 {
                    return Err("Image is too large".into());
                }

                if m.len() == 0 {
                    return Err("Image is empty".into());
                }
            }
            Err(e) => {
                return Err(
                    ("Fetching image metadata failed: ".to_string() + &e.to_string()).into(),
                );
            }
        };

        if partner.links.is_empty() {
            return Err("Links cannot be empty".into());
        }

        for link in &partner.links {
            if link.name.is_empty() {
                return Err("Link name cannot be empty".into());
            }

            if link.value.is_empty() {
                return Err("Link URL cannot be empty".into());
            }

            if !link.value.starts_with("https://") {
                return Err("Link URL must start with https://".into());
            }
        }

        // Check user id
        let user_exists = sqlx::query!(
            "SELECT user_id FROM users WHERE user_id = $1",
            partner.user_id
        )
        .fetch_optional(pool)
        .await?
        .is_some();

        if !user_exists {
            return Err("User does not exist".into());
        }

        Ok(())
    }

    match action {
        PartnerAction::List => {
            let prec = sqlx::query!(
                "SELECT id, name, short, links, type, created_at, user_id, bot_id FROM partners"
            )
            .fetch_all(&state.pool)
            .await
            .map_err(Error::new)?;

            let mut partners = Vec::new();

            for partner in prec {
                partners.push(Partner {
                    id: partner.id,
                    name: partner.name,
                    short: partner.short,
                    links: serde_json::from_value(partner.links).map_err(Error::new)?,
                    r#type: partner.r#type,
                    created_at: partner.created_at,
                    user_id: partner.user_id,
                    bot_id: partner.bot_id,
                })
            }

            let ptrec = sqlx::query!("SELECT id, name, short, icon, created_at FROM partner_types")
                .fetch_all(&state.pool)
                .await
                .map_err(Error::new)?;

            let mut partner_types = Vec::new();

            for partner_type in ptrec {
                partner_types.push(PartnerType {
                    id: partner_type.id,
                    name: partner_type.name,
                    short: partner_type.short,
                    icon: partner_type.icon,
                    created_at: partner_type.created_at,
                })
            }

            Ok((
                StatusCode::OK,
                Json(Partners {
                    partners,
                    partner_types,
                }),
            )
                .into_response())
        }
        PartnerAction::Create { partner } => {
            if !perms::has_perm(&user_perms, &"partners.create".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to create partners [partners.create]".to_string(),
                )
                    .into_response());
            }

            // Check if partner already exists
            let partner_exists = sqlx::query!("SELECT id FROM partners WHERE id = $1", partner.id)
                .fetch_optional(&state.pool)
                .await
                .map_err(Error::new)?
                .is_some();

            if partner_exists {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Partner already exists".to_string(),
                )
                    .into_response());
            }

            if let Err(e) = parse_partner(&state.pool, &partner).await {
                return Ok((StatusCode::BAD_REQUEST, e.to_string()).into_response());
            }

            // Insert partner
            sqlx::query!(
            "INSERT INTO partners (id, name, short, links, type, user_id, bot_id) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            partner.id,
            partner.name,
            partner.short,
            serde_json::to_value(partner.links).map_err(Error::new)?,
            partner.r#type,
            partner.user_id,
            partner.bot_id
        )
        .execute(&state.pool)
        .await
        .map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        PartnerAction::Update { partner } => {
            if !perms::has_perm(&user_perms, &"partners.update".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to update partners [partners.update]".to_string(),
                )
                    .into_response());
            }

            // Check if partner already exists
            let partner_exists = sqlx::query!("SELECT id FROM partners WHERE id = $1", partner.id)
                .fetch_optional(&state.pool)
                .await
                .map_err(Error::new)?
                .is_some();

            if !partner_exists {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Partner does not already exist".to_string(),
                )
                    .into_response());
            }

            if let Err(e) = parse_partner(&state.pool, &partner).await {
                return Ok((StatusCode::BAD_REQUEST, e.to_string()).into_response());
            }

            // Update partner
            sqlx::query!(
            "UPDATE partners SET name = $2, short = $3, links = $4, type = $5, user_id = $6, bot_id = $7 WHERE id = $1",
            partner.id,
            partner.name,
            partner.short,
            serde_json::to_value(partner.links).map_err(Error::new)?,
            partner.r#type,
            partner.user_id,
            partner.bot_id
        )
        .execute(&state.pool)
        .await
        .map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        PartnerAction::Delete { id } => {
            if !perms::has_perm(&user_perms, &"partners.delete".into()) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You do not have permission to delete partners [partners.delete]".to_string(),
                )
                    .into_response());
            }

            // Check if partner exists
            let partner_exists = sqlx::query!("SELECT id FROM partners WHERE id = $1", id)
                .fetch_optional(&state.pool)
                .await
                .map_err(Error::new)?
                .is_some();

            if !partner_exists {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Partner does not exist".to_string(),
                )
                    .into_response());
            }

            // Ensure that image has been uploaded to CDN
            // Get cdn path from cdn_scope hashmap
            let cdn_scopes = crate::config::CONFIG.panel.cdn_scopes.get();

            let Some(cdn_path) = cdn_scopes.get(&crate::config::CONFIG.panel.main_scope) else {
                return Ok(
                    (StatusCode::BAD_REQUEST, "Main scope not found".to_string()).into_response(),
                );
            };

            let path = format!("{}/partners/{}.webp", cdn_path.path, id);

            match std::fs::metadata(&path) {
                Ok(m) => {
                    if m.is_symlink() || m.is_file() {
                        // Delete the symlink
                        std::fs::remove_file(path).map_err(Error::new)?;
                    } else if m.is_dir() {
                        // Delete the directory
                        std::fs::remove_dir_all(path).map_err(Error::new)?;
                    }
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        return Ok((
                            StatusCode::BAD_REQUEST,
                            "Fetching asset metadata failed due to unknown error: ".to_string()
                                + &e.to_string(),
                        )
                            .into_response());
                    }
                }
            };

            sqlx::query!("DELETE FROM partners WHERE id = $1", id)
                .execute(&state.pool)
                .await
                .map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
    }
}

use std::collections::HashMap;

use crate::{panelapi::types::staff_disciplinary::StaffDisciplinaryType, Error};
use kittycat::perms::{PartialStaffPosition, Permission, StaffPermissions};
use num_traits::cast::ToPrimitive;
use sqlx::PgPool;

use super::types::{
    auth::AuthData, staff_disciplinary::StaffDisciplinary, staff_members::StaffMember,
    staff_positions::StaffPosition,
};

/// Checks auth, but does not ensure active sessions
pub async fn check_auth_insecure(pool: &PgPool, token: &str) -> Result<AuthData, Error> {
    // Delete expired auths
    sqlx::query!("DELETE FROM staffpanel__authchain WHERE created_at < NOW() - INTERVAL '1 hour'")
        .execute(pool)
        .await?;

    // Delete expired auths that are inactive
    sqlx::query!(
        "DELETE FROM staffpanel__authchain WHERE state = 'pending' AND created_at < NOW() - INTERVAL '5 minutes'"
    )
    .execute(pool)
    .await?;

    let count = sqlx::query!(
        "SELECT COUNT(*) FROM staffpanel__authchain WHERE token = $1",
        token
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap_or(0);

    if count == 0 {
        return Err("identityExpired".into());
    }

    let rec = sqlx::query!(
        "SELECT user_id, created_at, state FROM staffpanel__authchain WHERE token = $1",
        token
    )
    .fetch_one(pool)
    .await?;

    let prec = sqlx::query!(
        "SELECT positions FROM staff_members WHERE user_id = $1",
        rec.user_id
    )
    .fetch_optional(pool)
    .await?;

    let Some(positions) = prec else {
        return Err("identityExpired".into());
    };

    if positions.positions.is_empty() {
        return Err("identityExpired".into());
    }

    Ok(AuthData {
        user_id: rec.user_id,
        created_at: rec.created_at.timestamp(),
        state: rec.state,
    })
}

/// Checks auth, and ensures active sessions
///
/// Equivalent to `check_auth_insecure`, and rec.status != "active"
pub async fn check_auth(pool: &PgPool, token: &str) -> Result<AuthData, Error> {
    let rec = check_auth_insecure(pool, token).await?;

    if rec.state != "active" {
        return Err("sessionNotActive".into());
    }

    Ok(rec)
}

pub async fn get_staff_disciplinaries(
    pool: &PgPool,
    user_id: &str,
    active: bool,
) -> Result<Vec<StaffDisciplinary>, Error> {
    struct TRecord {
        id: String,
        created_at: chrono::DateTime<chrono::Utc>,
        expiry: Option<i64>,
        title: String,
        description: String,
        r#type: String,
    }

    let rec = {
        if active {
            let r = sqlx::query!(
                "SELECT id, created_at, EXTRACT(epoch FROM expiry) as expiry, title, description, type FROM staff_disciplinary WHERE user_id = $1 AND NOW() - created_at < expiry",
                user_id
            )
            .fetch_all(pool)
            .await?;

            let mut trec = Vec::new();

            for rec in r {
                trec.push(TRecord {
                    id: rec.id.hyphenated().to_string(),
                    created_at: rec.created_at,
                    expiry: rec.expiry.map(|d| {
                        // Convert to i64
                        d.to_i64().unwrap_or_default()
                    }),
                    title: rec.title,
                    description: rec.description,
                    r#type: rec.r#type,
                });
            }

            trec
        } else {
            let r = sqlx::query!(
                "SELECT id, created_at, EXTRACT(epoch FROM expiry) as expiry, title, description, type FROM staff_disciplinary WHERE user_id = $1",
                user_id
            )
            .fetch_all(pool)
            .await?;

            let mut trec = Vec::new();

            for rec in r {
                trec.push(TRecord {
                    id: rec.id.hyphenated().to_string(),
                    created_at: rec.created_at,
                    expiry: rec.expiry.map(|d| {
                        // Convert to i64
                        d.to_i64().unwrap_or_default()
                    }),
                    title: rec.title,
                    description: rec.description,
                    r#type: rec.r#type,
                });
            }

            trec
        }
    };

    let mut disc_type_cache: HashMap<String, StaffDisciplinaryType> = HashMap::new();
    let mut disciplinaries = Vec::new();

    for disciplinary in rec {
        let disciplinary_type = {
            if let Some(disc_type) = disc_type_cache.get(&disciplinary.r#type) {
                disc_type.clone()
            } else {
                let disc_type = sqlx::query!(
                    "SELECT name, description, self_assignable, perm_limits, additory, needs_approval,  EXTRACT(epoch FROM max_expiry) AS max_expiry FROM staff_disciplinary_types WHERE id = $1",
                    disciplinary.r#type
                )
                .fetch_one(pool)
                .await?;

                let dt = StaffDisciplinaryType {
                    id: disciplinary.r#type.clone(),
                    name: disc_type.name,
                    description: disc_type.description,
                    self_assignable: disc_type.self_assignable,
                    perm_limits: disc_type.perm_limits,
                    additory: disc_type.additory,
                    needs_approval: disc_type.needs_approval,
                    max_expiry: disc_type.max_expiry.map(|d| {
                        // Convert to f64
                        d.to_f64().unwrap_or_default()
                    }),
                    created_at: disciplinary.created_at,
                };

                disc_type_cache.insert(disciplinary.r#type.clone(), dt.clone());
                dt
            }
        };

        disciplinaries.push(StaffDisciplinary {
            id: disciplinary.id,
            user_id: user_id.to_string().clone(),
            created_at: disciplinary.created_at,
            expires_at: disciplinary.expiry,
            title: disciplinary.title,
            description: disciplinary.description,
            r#type: disciplinary_type,
        });
    }

    Ok(disciplinaries)
}

/// Returns the data of a staff member
pub async fn get_staff_member(
    pool: &PgPool,
    cache_http: &botox::cache::CacheHttpImpl,
    user_id: &str,
) -> Result<StaffMember, Error> {
    let data = sqlx::query!(
        "SELECT positions, perm_overrides, no_autosync, unaccounted, mfa_verified, created_at FROM staff_members WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e: sqlx::Error| format!("Error while getting staff perms of user {}: {}", user_id, e))?;

    let pos = sqlx::query!("SELECT id, name, role_id, perms, corresponding_roles, icon, index, created_at FROM staff_positions WHERE id = ANY($1)", &data.positions)
    .fetch_all(pool)
    .await
    .map_err(|e: sqlx::Error| format!("Error while getting positions of user {}: {}", user_id, e))?;

    let mut positions = Vec::new();
    let sp = StaffPermissions {
        user_positions: pos
            .iter()
            .map(|p| PartialStaffPosition {
                id: p.id.hyphenated().to_string(),
                index: p.index,
                perms: p
                    .perms
                    .iter()
                    .map(|x| Permission::from_string(x))
                    .collect::<Vec<Permission>>(),
            })
            .collect(),
        perm_overrides: data
            .perm_overrides
            .iter()
            .map(|x| Permission::from_string(x))
            .collect::<Vec<Permission>>(),
    };

    for position_data in pos {
        positions.push(StaffPosition {
            id: position_data.id.hyphenated().to_string(),
            name: position_data.name,
            role_id: position_data.role_id,
            perms: position_data.perms,
            corresponding_roles: serde_json::from_value(position_data.corresponding_roles.clone())
                .unwrap_or_default(),
            icon: position_data.icon,
            index: position_data.index,
            created_at: position_data.created_at,
        });
    }

    let disciplinaries = get_staff_disciplinaries(pool, user_id, true).await?;

    let resolved_perms = {
        if disciplinaries.is_empty() {
            let sp = sp.clone();
            sp.resolve()
        } else {
            let mut virtual_sp = sp.clone();
            let mut added_ids = Vec::new();
            for disc in &disciplinaries {
                // Add oermissions to virtual_sp as a index 0 position
                virtual_sp.user_positions.push(PartialStaffPosition {
                    id: disc.id.clone(),
                    index: 0,
                    perms: disc
                        .r#type
                        .perm_limits
                        .iter()
                        .map(|x| Permission::from_string(x))
                        .collect::<Vec<Permission>>(),
                });
                added_ids.push(disc.id.clone());

                if !disc.r#type.additory {
                    // Remove all not in added_ids
                    virtual_sp
                        .user_positions
                        .retain(|p| added_ids.contains(&p.id));
                }
            }

            virtual_sp.resolve()
        }
    };

    Ok(StaffMember {
        user_id: user_id.to_string().clone(),
        user: crate::impls::dovewing::get_platform_user(
            pool,
            crate::impls::dovewing::DovewingSource::Discord(cache_http.clone()),
            user_id,
        )
        .await?,
        positions,
        disciplinaries,
        perm_overrides: data.perm_overrides,
        resolved_perms_kc: resolved_perms.iter().map(|x| x.to_string()).collect(),
        resolved_perms,
        staff_permission: sp,
        no_autosync: data.no_autosync,
        unaccounted: data.unaccounted,
        mfa_verified: data.mfa_verified,
        created_at: data.created_at,
    })
}

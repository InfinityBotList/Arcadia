use std::str::FromStr;

use crate::impls::target_types::TargetType;
use crate::panelapi::auth::{check_auth, get_staff_member};
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::webcore::{CoreConstants, Hello, InstanceConfig, PanelServers};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use strum::VariantNames;

const HELLO_VERSION: u16 = 5;

pub async fn hello(
    state: &AppState,
    // Login token
    login_token: String,
    // Authorize protocol version, should be `AUTH_VERSION`
    version: u16,
) -> Result<Response, Error> {
    let auth_data = check_auth(&state.pool, &login_token)
        .await
        .map_err(Error::new)?;

    if version != HELLO_VERSION {
        return Ok((StatusCode::BAD_REQUEST, "Invalid version".to_string()).into_response());
    }

    // Get permissions
    let staff_member = get_staff_member(&state.pool, &state.cache_http, &auth_data.user_id)
        .await
        .map_err(Error::new)?;

    let mut target_types: Vec<TargetType> = Vec::new();

    for target_type in TargetType::VARIANTS {
        let variant = TargetType::from_str(target_type).map_err(Error::new)?;
        target_types.push(variant);
    }

    Ok((
    StatusCode::OK,
    Json(
        Hello {
            instance_config: InstanceConfig {
                description: {
                    if *crate::config::CURRENT_ENV == "staging" {
                        "Arcadia Staging Panel Instance".to_string()
                    } else {
                        "Arcadia Production Panel Instance".to_string()
                    }
                },
                warnings: vec![
                    "Oh, hello there. This panel is currently being rewritten, and may have some issues. If you find any issues, please contact a Lead Developer in the `Staff Center` Discord Server!".to_string(),
                    "[Warning]: `panel.infinitybots.gg` will soon be unaccessible as we move our panel into the main site.".to_string()
                ],
            },
            auth_data,
            staff_member,
            core_constants: CoreConstants {
                frontend_url: crate::config::CONFIG.frontend_url.get().clone(),
                infernoplex_url: crate::config::CONFIG.infernoplex_url.clone(),
                popplio_url: crate::config::CONFIG.popplio_url.clone(),
                htmlsanitize_url: crate::config::CONFIG.htmlsanitize_url.clone(),
                cdn_url: crate::config::CONFIG.cdn_url.clone(),
                servers: PanelServers {
                    main: crate::config::CONFIG.servers.main.to_string(),
                    staff: crate::config::CONFIG.servers.staff.to_string(),
                    testing: crate::config::CONFIG.servers.testing.to_string(),
                },
            },
            target_types,
        }
    )
)
    .into_response())
}

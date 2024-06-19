use crate::panelapi::auth::{check_auth, check_auth_insecure};
use crate::panelapi::core::{AppState, Error};
use crate::panelapi::types::auth::{AuthorizeAction, MfaLogin, MfaLoginSecret};
use crate::panelapi::types::webcore::StartAuth;
use axum::response::Response;
use axum::{http::StatusCode, response::IntoResponse, Json};
use rand::Rng;
use serde::Deserialize;
use serenity::all::User;
use std::time::Duration;

const AUTH_VERSION: u16 = 5;

pub async fn authorize(
    state: &AppState,
    // Authorize protocol version, should be `AUTH_VERSION`
    version: u16,
    // Action to take
    action: AuthorizeAction,
) -> Result<Response, Error> {
    if version != AUTH_VERSION {
        return Ok((StatusCode::BAD_REQUEST, "Invalid version".to_string()).into_response());
    }

    match action {
        AuthorizeAction::Begin {
            scope,
            redirect_url,
        } => {
            if scope != crate::config::CONFIG.panel.panel_scope {
                return Ok((StatusCode::BAD_REQUEST, "Invalid scope".to_string()).into_response());
            }

            Ok(
                (
                    StatusCode::OK,
                    Json(
                        StartAuth {
                            login_url: format!(
                                "https://discord.com/api/oauth2/authorize?client_id={client_id}&redirect_uri={redirect_url}&response_type=code&scope=identify",
                                client_id = crate::config::CONFIG.panel.client_id,
                                redirect_url = redirect_url
                            ),
                            scope: crate::config::CONFIG.panel.panel_scope.clone(),
                            response_scope: crate::config::CONFIG.panel.panel_response_scope.clone(),
                        }
                    )
                ).into_response()
            )
        }
        AuthorizeAction::CreateSession { code, redirect_url } => {
            if !crate::config::CONFIG
                .panel
                .redirect_url
                .contains(&redirect_url)
            {
                return Ok(
                    (StatusCode::BAD_REQUEST, "Invalid redirect url".to_string()).into_response(),
                );
            }

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(Error::new)?;

            let resp = client
                .post("https://discord.com/api/oauth2/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("User-Agent", "DiscordBot (arcadia v1.0)")
                .form(&[
                    ("client_id", crate::config::CONFIG.panel.client_id.as_str()),
                    (
                        "client_secret",
                        crate::config::CONFIG.panel.client_secret.as_str(),
                    ),
                    ("grant_type", "authorization_code"),
                    ("code", code.as_str()),
                    ("redirect_uri", redirect_url.as_str()),
                    ("scope", "identify"),
                ])
                .send()
                .await
                .map_err(Error::new)?
                .error_for_status()
                .map_err(Error::new)?;

            #[derive(Deserialize)]
            struct Oauth2 {
                access_token: String,
            }

            let oauth2 = resp.json::<Oauth2>().await.map_err(Error::new)?;

            let user_resp = client
                .get("https://discord.com/api/users/@me")
                .header(
                    "Authorization",
                    "Bearer ".to_string() + oauth2.access_token.as_str(),
                )
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("User-Agent", "DiscordBot (arcadia v1.0)")
                .send()
                .await
                .map_err(Error::new)?
                .error_for_status()
                .map_err(Error::new)?;

            let user = user_resp.json::<User>().await.map_err(Error::new)?;

            let rec = sqlx::query!(
                "SELECT positions FROM staff_members WHERE user_id = $1",
                user.id.to_string()
            )
            .fetch_optional(&state.pool)
            .await
            .map_err(Error::new)?;

            let Some(positions) = rec else {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You are not a staff member [not in db]".to_string(),
                )
                    .into_response());
            };

            if positions.positions.is_empty() {
                return Ok((
                    StatusCode::FORBIDDEN,
                    "You are not a staff member [no positions]".to_string(),
                )
                    .into_response());
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            sqlx::query!(
                "DELETE FROM staffpanel__authchain WHERE user_id = $1",
                user.id.to_string()
            )
            .execute(&mut *tx)
            .await
            .map_err(Error::new)?;

            // Create a random number between 4196 and 6000 for the token
            let tlength = rand::thread_rng().gen_range(4196..6000);

            let token = botox::crypto::gen_random(tlength as usize);

            sqlx::query!(
                "INSERT INTO staffpanel__authchain (user_id, token, popplio_token, state) VALUES ($1, $2, $3, $4)",
                user.id.to_string(),
                token,
                botox::crypto::gen_random(2048),
                "pending"
            )
            .execute(&mut *tx)
            .await
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::OK, token).into_response())
        }
        AuthorizeAction::CheckMfaState { login_token } => {
            let auth_data = check_auth_insecure(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            if auth_data.state != "pending" && auth_data.state != "active" {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "This endpoint can only be used by pending and active sessions"
                        .to_string(),
                });
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            let mfa = sqlx::query!(
                "SELECT mfa_secret, mfa_verified FROM staff_members WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| {
                Error::new(format!(
                    "Failed to fetch staff member mfa_secret/mfa_verified: {}",
                    e
                ))
            })?;

            if mfa.is_none() {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "You are not a staff member".to_string(),
                });
            }

            let mfa = mfa.unwrap();

            if mfa.mfa_secret.is_none() || !mfa.mfa_verified {
                let temp_secret = thotp::generate_secret(160);

                let temp_secret_enc = thotp::encoding::encode(&temp_secret, data_encoding::BASE32);

                sqlx::query!(
                    "UPDATE staff_members SET mfa_secret = $1 WHERE user_id = $2",
                    &temp_secret_enc,
                    auth_data.user_id,
                )
                .execute(&mut *tx)
                .await
                .map_err(Error::new)?;

                let qr_code_uri = thotp::qr::otp_uri(
                    // Type of otp
                    "totp",
                    // The encoded secret
                    &temp_secret_enc,
                    // Your big corp title
                    "staff@infinitybots.gg",
                    // Your big corp issuer
                    "Infinity List",
                    // The counter (Only HOTP)
                    None,
                )
                .map_err(Error::new)?;

                let qr = thotp::qr::generate_code_svg(
                    &qr_code_uri,
                    // The qr code width (None defaults to 200)
                    None,
                    // The qr code height (None defaults to 200)
                    None,
                    // Correction level, M is the default
                    thotp::qr::EcLevel::M,
                )
                .map_err(Error::new)?;

                tx.commit().await.map_err(Error::new)?;

                Ok((
                    StatusCode::OK,
                    Json(MfaLogin {
                        info: Some(MfaLoginSecret {
                            qr_code: qr,
                            otp_url: qr_code_uri,
                            secret: temp_secret_enc,
                        }),
                    }),
                )
                    .into_response())
            } else {
                tx.rollback().await.map_err(Error::new)?;

                Ok((StatusCode::OK, Json(MfaLogin { info: None })).into_response())
            }
        }
        AuthorizeAction::ResetMfaTotp { login_token, otp } => {
            let auth_data = check_auth(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            let secret = sqlx::query!(
                "SELECT mfa_secret FROM staff_members WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(Error::new)?
            .mfa_secret;

            if secret.is_none() {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "mfaNotSetup".to_string(),
                });
            }

            let secret = thotp::encoding::decode(&secret.unwrap(), data_encoding::BASE32)
                .map_err(Error::new)?;

            let (result, _discrepancy) = thotp::verify_totp(&otp, &secret, 0).unwrap();

            if !result {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Invalid OTP Entered".to_string(),
                });
            }

            sqlx::query!(
                "UPDATE staff_members SET mfa_secret = NULL, mfa_verified = FALSE WHERE user_id = $1",
                auth_data.user_id
            )
            .execute(&mut *tx)
            .await
            .map_err(Error::new)?;

            // Revoke existing sessions
            sqlx::query!(
                "DELETE FROM staffpanel__authchain WHERE user_id = $1",
                auth_data.user_id
            )
            .execute(&mut *tx)
            .await
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        AuthorizeAction::ActivateSession { login_token, otp } => {
            let auth_data = check_auth_insecure(&state.pool, &login_token)
                .await
                .map_err(Error::new)?;

            if auth_data.state != "pending" {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "sessionAlreadyActive".to_string(),
                });
            }

            let mut tx = state.pool.begin().await.map_err(Error::new)?;

            let mfa = sqlx::query!(
                "SELECT mfa_secret, mfa_verified FROM staff_members WHERE user_id = $1",
                auth_data.user_id
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(Error::new)?;

            if mfa.mfa_secret.is_none() {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "mfaNotSetup".to_string(),
                });
            }

            let secret = thotp::encoding::decode(&mfa.mfa_secret.unwrap(), data_encoding::BASE32)
                .map_err(Error::new)?;

            let (result, _discrepancy) = thotp::verify_totp(&otp, &secret, 0).unwrap();

            if !result {
                return Err(Error {
                    status: StatusCode::BAD_REQUEST,
                    message: "Invalid OTP entered".to_string(),
                });
            }

            sqlx::query!(
                "UPDATE staffpanel__authchain SET state = 'active' WHERE token = $1",
                login_token
            )
            .execute(&mut *tx)
            .await
            .map_err(Error::new)?;

            sqlx::query!(
                "UPDATE staff_members SET mfa_verified = TRUE WHERE user_id = $1",
                auth_data.user_id
            )
            .execute(&mut *tx)
            .await
            .map_err(Error::new)?;

            tx.commit().await.map_err(Error::new)?;

            Ok((StatusCode::NO_CONTENT, "").into_response())
        }
        AuthorizeAction::Logout { login_token } => {
            // Just delete the auth, no point in even erroring if it doesn't exist
            let row = sqlx::query!(
                "DELETE FROM staffpanel__authchain WHERE token = $1",
                login_token
            )
            .execute(&state.pool)
            .await
            .map_err(Error::new)?;

            Ok((StatusCode::OK, row.rows_affected().to_string()).into_response())
        }
    }
}

use actix_web::{http::header::HeaderValue, post, web, HttpRequest, HttpResponse};

use crate::models::{RPCMethod, RPCRequest};

/// Web RPC API for the Staff/Admin Panel
#[post("/")]
pub async fn web_rpc_api(req: HttpRequest, info: web::Json<RPCRequest>) -> HttpResponse {
    let data: &crate::models::AppState = req
        .app_data::<web::Data<crate::models::AppState>>()
        .unwrap();

    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();

    let check = sqlx::query!(
        "SELECT staff, ibldev, iblhdev, admin, hadmin, api_token FROM users WHERE user_id = $1",
        &info.user_id
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().body("User not found");
    }

    let check = check.unwrap();

    if check.api_token != auth {
        return HttpResponse::Unauthorized().body("Invalid token");
    }

    if !check.staff {
        return HttpResponse::Unauthorized().body("Staff-only endpoint");
    }

    match &info.method {
        RPCMethod::BotApprove { bot_id } => {
            let res = libavacado::staff::approve_bot(
                &data.cache_http,
                &data.pool,
                &bot_id,
                &info.user_id,
                &info.reason,
            )
            .await;

            if res.is_err() {
                HttpResponse::BadRequest().body(res.unwrap_err().to_string())
            } else {
                HttpResponse::Ok().body(res.unwrap().invite)
            }
        }
        RPCMethod::BotDeny { bot_id } => {
            let err = libavacado::staff::deny_bot(
                &data.cache_http,
                &data.pool,
                &bot_id,
                &info.user_id,
                &info.reason,
            )
            .await;

            if err.is_err() {
                HttpResponse::BadRequest().body(err.unwrap_err().to_string())
            } else {
                HttpResponse::NoContent().finish()
            }
        }
        RPCMethod::BotVoteReset { bot_id } => {
            if !(check.hadmin || check.iblhdev) {
                HttpResponse::Unauthorized().body("Permission denied")
            } else {
                let err = libavacado::manage::vote_reset_bot(
                    &data.cache_http,
                    &data.pool,
                    &bot_id,
                    &info.user_id,
                    &info.reason,
                )
                .await;

                if err.is_err() {
                    HttpResponse::BadRequest().body(err.unwrap_err().to_string())
                } else {
                    HttpResponse::NoContent().finish()
                }
            }
        }
        RPCMethod::BotVoteResetAll {} => {
            if !(check.hadmin || check.iblhdev) {
                HttpResponse::Unauthorized().body("Permission denied")
            } else {
                let err = libavacado::manage::vote_reset_all_bot(
                    &data.cache_http,
                    &data.pool,
                    &info.user_id,
                    &info.reason,
                )
                .await;

                if err.is_err() {
                    HttpResponse::BadRequest().body(err.unwrap_err().to_string())
                } else {
                    HttpResponse::NoContent().finish()
                }
            }
        },
        RPCMethod::BotUnverify { bot_id } => {
            if !(check.hadmin || check.iblhdev) {
                HttpResponse::Unauthorized().body("Permission denied")
            } else {
                let err = libavacado::manage::unverify_bot(
                    &data.cache_http,
                    &data.pool,
                    &bot_id,
                    &info.user_id,
                    &info.reason,
                )
                .await;
            
                if err.is_err() {
                    HttpResponse::BadRequest().body(err.unwrap_err().to_string())
                } else {
                    HttpResponse::NoContent().finish()
                }
            }
        },
    }
}
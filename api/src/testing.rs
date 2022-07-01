use actix_web::{post, HttpRequest, HttpResponse, http::header::HeaderValue, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ApproveDenyRequest {
    staff_id: String,
    bot_id: String,
    reason: String,
}

#[post("/approve")]
pub async fn approve(req: HttpRequest, info: web::Json<ApproveDenyRequest>) -> HttpResponse {
    let data: &crate::models::AppState = req.app_data::<web::Data<crate::models::AppState>>().unwrap();

    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();

    let check = sqlx::query!(
        "SELECT staff, api_token FROM users WHERE user_id = $1",
        &info.staff_id
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let check = check.unwrap();
    
    if check.api_token != auth || !check.staff {
        return HttpResponse::Unauthorized().finish();
    }

    let err = libavacado::staff::approve_bot(&data.cache_http, &data.pool, &info.bot_id, &info.staff_id, &info.reason).await;

    if err.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: err.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().body("")
}
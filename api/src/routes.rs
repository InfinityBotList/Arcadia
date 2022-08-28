use actix_web::{get, http::header::HeaderValue, post, web, HttpRequest, HttpResponse};
use libavacado::search::{SearchOpts, SearchFilter};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Request {
    staff_id: String,
    bot_id: String,
    reason: String,
}

#[derive(Deserialize)]
pub struct GenericRequest {
    staff_id: String,
    reason: String,
}

#[post("/approve")]
pub async fn approve(req: HttpRequest, info: web::Json<Request>) -> HttpResponse {
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

    let err = libavacado::staff::approve_bot(
        &data.cache_http,
        &data.pool,
        &info.bot_id,
        &info.staff_id,
        &info.reason,
    )
    .await;

    if err.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: err.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().body("")
}

#[post("/deny")]
pub async fn deny(req: HttpRequest, info: web::Json<Request>) -> HttpResponse {
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

    let err = libavacado::staff::deny_bot(
        &data.cache_http,
        &data.pool,
        &info.bot_id,
        &info.staff_id,
        &info.reason,
    )
    .await;

    if err.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: err.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().body("")
}

#[post("/votes-reset")]
pub async fn vote_reset(req: HttpRequest, info: web::Json<Request>) -> HttpResponse {
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
        "SELECT iblhdev, hadmin, api_token FROM users WHERE user_id = $1",
        &info.staff_id.to_string()
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let check = check.unwrap();

    if check.api_token != auth || !(check.hadmin || check.iblhdev) {
        return HttpResponse::Unauthorized().finish();
    }

    let err = libavacado::manage::vote_reset(
        &data.cache_http,
        &data.pool,
        &info.bot_id.to_string(),
        &info.staff_id.to_string(),
        &info.reason,
    )
    .await;

    if err.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: err.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().body("")
}

#[post("/votes-reset/all")]
pub async fn vote_reset_all(req: HttpRequest, info: web::Json<GenericRequest>) -> HttpResponse {
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
        "SELECT iblhdev, hadmin, api_token FROM users WHERE user_id = $1",
        &info.staff_id.to_string()
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let check = check.unwrap();

    if check.api_token != auth || !(check.hadmin || check.iblhdev) {
        return HttpResponse::Unauthorized().finish();
    }

    let err = libavacado::manage::vote_reset_all(
        &data.cache_http,
        &data.pool,
        &info.staff_id.to_string(),
        &info.reason,
    )
    .await;

    if err.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: err.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().body("")
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
    gc_from: Option<i32>,
    gc_to: Option<i32>,
    votes_from: Option<i32>,
    votes_to: Option<i32>,
}

#[get("/tetanus")]
pub async fn tetanus_search_service(
    req: HttpRequest, 
    q: web::Query<SearchQuery>,
) -> HttpResponse {
    let data: &crate::models::AppState = req
        .app_data::<web::Data<crate::models::AppState>>()
        .unwrap();

    let search_res = libavacado::search::search_bots(&q.q, &data.pool, &data.avacado_public, &SearchOpts {
        gc: SearchFilter {
            from: q.gc_from,
            to: q.gc_to,
        },
        votes: SearchFilter {
            from: q.votes_from,
            to: q.votes_to,
        },
    }).await;

    if search_res.is_err() {
        let err = search_res.unwrap_err();
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: err.to_string(),
            context: None,
        });
    }

    let search_res = search_res.unwrap();

    HttpResponse::Ok().json(search_res)
}

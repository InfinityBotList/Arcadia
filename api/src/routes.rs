use actix_web::{get, http::header::HeaderValue, post, web, HttpRequest, HttpResponse};
use libavacado::search::{SearchFilter, SearchOpts};
use serde::{Deserialize, Serialize};

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
    servers_from: Option<i32>,
    servers_to: Option<i32>,
}

#[get("/tetanus")]
pub async fn tetanus_search_service(req: HttpRequest, q: web::Query<SearchQuery>) -> HttpResponse {
    let data: &crate::models::AppState = req
        .app_data::<web::Data<crate::models::AppState>>()
        .unwrap();

    let search_res = libavacado::search::search_bots(
        &q.q,
        &data.pool,
        &data.avacado_public,
        &SearchOpts {
            gc: SearchFilter {
                from: q.gc_from,
                to: q.gc_to,
            },
            votes: SearchFilter {
                from: q.votes_from,
                to: q.votes_to,
            },
            servers: SearchFilter {
                from: q.servers_from,
                to: q.servers_to,
            },
        },
    )
    .await;

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

/// Get all current maintenances
#[get("/maints")]
pub async fn get_current_maints(_req: HttpRequest) -> HttpResponse {
    let maints = libavacado::public::maint_status();

    if let Ok(maints) = maints {
        return HttpResponse::Ok().json(maints);
    }

    HttpResponse::BadRequest().json(crate::models::APIResponse {
        done: false,
        reason: maints.err().unwrap().to_string(),
        context: None,
    })
}

#[derive(Serialize, Deserialize)]
pub struct SVQuery {
    uid: String,
    frag: String,
}

#[derive(Serialize, Deserialize)]
pub struct SVODQuery {
    code: String,
}

/// Get onboarding response data
#[get("/svapi-onboarddata")]
pub async fn staff_verify_onboard_data_api(
    req: HttpRequest,
    q: web::Query<SVODQuery>,
) -> HttpResponse {
    let data: &crate::models::AppState = req
        .app_data::<web::Data<crate::models::AppState>>()
        .unwrap();

    // Check SVAPI version
    let svapi_header = req.headers().get("sv-version");

    if svapi_header.is_none() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "SVSession expired".to_string(),
            context: None,
        });
    }

    let svapi_header = svapi_header.unwrap().to_str().unwrap();

    if svapi_header != "wistala3" {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "SVSession expired".to_string(),
            context: None,
        });
    }

    let data = sqlx::query!(
        "SELECT user_id, data FROM onboard_data WHERE onboard_code = $1",
        &q.code
    )
    .fetch_one(&data.pool)
    .await;

    if data.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "SVSession expired".to_string(),
            context: None,
        });
    }

    let rec = data.unwrap();

    let mut data = rec.data;

    data["user_id"] = sqlx::types::JsonValue::String(rec.user_id);

    HttpResponse::Ok().json(data)
}

/// Staff Verify Code Fetch API
#[get("/svapi")]
pub async fn staff_verify_fetch_api(req: HttpRequest, q: web::Query<SVQuery>) -> HttpResponse {
    let data: &crate::models::AppState = req
        .app_data::<web::Data<crate::models::AppState>>()
        .unwrap();

    let code = sqlx::query!(
        "SELECT staff_onboard_session_code FROM users WHERE user_id = $1",
        &q.uid
    )
    .fetch_one(&data.pool)
    .await;

    if code.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "User not found".to_string(),
            context: None,
        });
    }

    let code = code.unwrap().staff_onboard_session_code;

    if code.is_none() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "SVSession not found".to_string(),
            context: None,
        });
    }

    let code = code.unwrap();

    // Get first 20 chars of code (fragment code)
    let frcode = &code[..20];

    if frcode != q.frag {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Invalid SVSession".to_string(),
            context: None,
        });
    }

    // Split code by @
    let codesplit = code.split('@').collect::<Vec<&str>>();

    if codesplit.len() != 2 {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Invalid SVSession".to_string(),
            context: None,
        });
    }

    let time_nonce = codesplit[1];
    let time_nonce = time_nonce.parse::<i64>();

    if time_nonce.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Invalid SVSession".to_string(),
            context: None,
        });
    }

    let time_nonce = time_nonce.unwrap();

    // Get current time and subtract from time_nonce
    let now = chrono::Utc::now().timestamp();

    if now - time_nonce > 3600 {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "SVSession expired".to_string(),
            context: None,
        });
    }

    // Check SVAPI version
    let svapi_header = req.headers().get("sv-version");

    if svapi_header.is_none() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "SVSession expired".to_string(),
            context: None,
        });
    }

    let svapi_header = svapi_header.unwrap().to_str().unwrap();

    if svapi_header != "pika9" {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "SVSession expired".to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().json(crate::models::APIResponse {
        done: true,
        reason: codesplit[0].to_string(),
        context: None,
    })
}

#[derive(Serialize)]
pub struct SeedList {
    name: &'static str,
    id: &'static str,
    url: &'static str,
}

/// Get a list of all links for seedguide index
#[get("/seedlist")]
pub async fn seedlist(_req: HttpRequest) -> HttpResponse {
    const SEEDLIST: &[SeedList] = &[
        SeedList {
            name: "Index",
            id: "index",
            url: "/",
        },
        SeedList {
            name: "Staff Guide",
            id: "sguide",
            url: "/sovngarde",
        },
        SeedList {
            name: "Dev Changelog",
            id: "changelog",
            url: "/changelog",
        },
        SeedList {
            name: "Dev Seed Guide",
            id: "devseedguide",
            url: "/seed",
        },
    ];

    HttpResponse::Ok().json(SEEDLIST)
}

/// Returns a a staff/dev application form
#[get("/herpes")]
pub async fn get_apps_api(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(libavacado::staffapps::get_apps())
}
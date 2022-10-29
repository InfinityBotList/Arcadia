use actix_web::{get, http::header::HeaderValue, post, web, HttpRequest, HttpResponse};
use libavacado::{search::{SearchFilter, SearchOpts}, types::{StaffAppResponse, CreateBot}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Deserialize)]
pub struct UserRequest {
    user_id: String,
}

#[derive(Deserialize)]
pub struct CreateAppQuery {
    user_id: String,
    position: String
}

#[derive(Deserialize)]
pub struct GetAppQuery {
    app_id: String,
    user_id: String,
}

#[post("/rindfleischetikettierungsueberwachungsaufgabenuebertragungsgesetherpacyphygohnalaids/approve")]
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

#[post("/rindfleischetikettierungsueberwachungsaufgabenuebertragungsgesetherpacyphygohnalaids/deny")]
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

#[post("/rindfleischetikettierungsueberwachungsaufgabenuebertragungsgesetherpacyphygohnalaids/votes-reset")]
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

#[post("/rindfleischetikettierungsueberwachungsaufgabenuebertragungsgesetherpacyphygohnalaids/votes-reset/all")]
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
    gc: Option<SearchFilter>,
    votes: Option<SearchFilter>,
    servers: Option<SearchFilter>,
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
            gc: q.gc.unwrap_or_default(),
            votes: q.votes.unwrap_or_default(),
            servers: q.servers.unwrap_or_default(),
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

/// Returns a staff/dev application form
#[get("/herpes")]
pub async fn get_apps_api(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(libavacado::staffapps::get_apps())
}

/// Returns the interview questions form
#[get("/herpes/zoster")]
pub async fn get_interview_api(_req: HttpRequest) -> HttpResponse {
   HttpResponse::Ok().json(libavacado::staffapps::get_interview_questions())
}

/// Finalizes the application
#[post("/herpes/zoster")]
pub async fn finalize_app_api(
    req: HttpRequest, 
    info: web::Query<GetAppQuery>,
    body: web::Json<HashMap<String, String>>,
) -> HttpResponse {
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

    let info = info.into_inner();

    let check = sqlx::query!(
        "SELECT api_token FROM users WHERE user_id = $1",
        &info.user_id.to_string()
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let check = check.unwrap();

    if check.api_token != auth {
        return HttpResponse::Unauthorized().finish();
    }

    let app = libavacado::staffapps::finalize_app(
        &data.avacado_public,
        &data.pool,
        &info.app_id,
        body.into_inner()
    ).await;

    if app.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: app.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().finish()
}


#[post("/herpes")]
pub async fn create_app_api(
    req: HttpRequest, 
    info: web::Query<CreateAppQuery>,
    body: web::Json<HashMap<String, String>>
) -> HttpResponse {
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
    
    let info = info.into_inner();
    
    let check = sqlx::query!(
        "SELECT api_token FROM users WHERE user_id = $1",
        &info.user_id.to_string()
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let check = check.unwrap();

    if check.api_token != auth {
        return HttpResponse::Unauthorized().finish();
    }

    let app = libavacado::staffapps::create_app(
        &data.avacado_public,
        &data.pool,
        &info.user_id,
        &info.position,
        body.into_inner()
    ).await;

    if app.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: app.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().finish()
}

/// Returns a callback URL for app site
#[get("/herpes/auth")]
pub async fn get_apps_auth_api(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().body(
        format!("https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify", std::env::var("APP_SITE_CLIENT_ID").unwrap(), std::env::var("APP_SITE_REDIRECT_URL").unwrap())
    )
}

/// Performs oauth2 callback for app site
#[get("/herpes/callback")]
pub async fn perform_apps_auth_api(req: HttpRequest, data: web::Query<crate::models::OauthReq>) -> HttpResponse {
    // Get access token using reqwest
    let client = reqwest::Client::new();

    let data = data.into_inner();

    let res = client
        .post("https://discord.com/api/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "client_id={}&client_secret={}&grant_type=authorization_code&code={}&redirect_uri={}&scope=identify",
            std::env::var("APP_SITE_CLIENT_ID").unwrap(),
            std::env::var("APP_SITE_CLIENT_SECRET").unwrap(),
            data.code,
            std::env::var("APP_SITE_REDIRECT_URL").unwrap(),
        ))
        .send()
        .await;
    
    if res.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to get access token".to_string(),
            context: None,
        });
    }

    let res = res.unwrap();

    if res.status() != 200 {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to get access token with status code".to_string() + &res.status().to_string(),
            context: None,
        });
    }

    let res = res.json::<crate::models::OauthRes>().await;

    if res.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to get access token".to_string(),
            context: None,
        });
    }

    let res = res.unwrap();

    // Get user ID using access token
    let res = client
        .get("https://discord.com/api/users/@me")
        .header("Authorization", format!("Bearer {}", res.access_token))
        .send()
        .await;
    
    if res.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to get user ID".to_string(),
            context: None,
        });
    }

    let res = res.unwrap();

    if res.status() != 200 {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to get user ID with status code".to_string() + &res.status().to_string(),
            context: None,
        });
    }

    let res = res.json::<crate::models::OauthUser>().await;

    if res.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to get user ID".to_string(),
            context: None,
        });
    }

    let res = res.unwrap();

    let app_state: &crate::models::AppState = req
    .app_data::<web::Data<crate::models::AppState>>()
    .unwrap();

    let row = sqlx::query!("SELECT api_token FROM users WHERE user_id = $1", res.id)
        .fetch_one(&app_state.pool)
        .await;
    
    if row.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to get api token, try logging in on the main site?".to_string(),
            context: None,
        });
    }

    let row = row.unwrap();

    let redirect = format!("https://{}/login/callback?user_id={}&api_token={}", data.state, res.id, row.api_token);
    
    HttpResponse::TemporaryRedirect().append_header(("Location", redirect)).finish()
}

#[get("/herpes/app")]
pub async fn get_app_api(req: HttpRequest, info: web::Query<GetAppQuery>) -> HttpResponse {
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
        "SELECT api_token FROM users WHERE user_id = $1",
        &info.user_id
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let check = check.unwrap();

    if check.api_token != auth {
        return HttpResponse::Unauthorized().finish();
    }
    

    let row = sqlx::query!(
        "SELECT app_id, user_id, position, answers, interview_answers, state, created_at, likes, dislikes FROM apps WHERE app_id = $1",
        info.app_id
    )
    .fetch_one(&data.pool)
    .await;

    if row.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to get app".to_string(),
            context: None,
        });
    }

    let row = row.unwrap();

    let mut likes = Vec::new();

    for like in row.likes {
        likes.push(like.to_string());
    }

    let mut dislikes = Vec::new();

    for dislike in row.dislikes {
        dislikes.push(dislike.to_string());
    }

    HttpResponse::Ok().json(StaffAppResponse {
        user_id: row.user_id,
        app_id: row.app_id,
        created_at: row.created_at,
        answers: row.answers,
        interview: row.interview_answers,
        position: row.position,
        state: row.state,
        likes,
        dislikes,
    })
}

/// Selects a candidate for a interview
#[post("/herpes/app/interview")]
pub async fn send_interview_api(req: HttpRequest, info: web::Query<GetAppQuery>) -> HttpResponse {
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
        "SELECT api_token, iblhdev, hadmin FROM users WHERE user_id = $1",
        &info.user_id
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
    

    let row = sqlx::query!(
        "SELECT COUNT(1) FROM apps WHERE app_id = $1",
        info.app_id
    )
    .fetch_one(&data.pool)
    .await;

    if row.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to find app".to_string(),
            context: None,
        });
    }

    let row = row.unwrap();

    if row.count.unwrap_or(0) == 0 {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to find app".to_string(),
            context: None,
        });
    }

    let err = libavacado::staffapps::send_interview(&data.avacado_public, &data.pool, &info.app_id).await;

    if err.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: "Failed to send interview".to_string() + &err.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().json(crate::models::APIResponse {
        done: true,
        reason: "Sent interview".to_string(),
        context: None,
    })
}


/// Returns a list of staff applications that have been made
#[get("/herpes/list")]
pub async fn get_app_list(req: HttpRequest, info: web::Query<UserRequest>) -> HttpResponse {
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
    
    let info = info.into_inner();
    
    let check = sqlx::query!(
        "SELECT iblhdev, hadmin, ibldev, admin, api_token FROM users WHERE user_id = $1",
        &info.user_id.to_string()
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let check = check.unwrap();

    if check.api_token != auth || !(check.hadmin || check.iblhdev || check.ibldev || check.admin) {
        return HttpResponse::Unauthorized().finish();
    }

    let req = libavacado::staffapps::get_made_apps(&data.pool).await;

    if req.is_err() {
        return HttpResponse::InternalServerError().json(crate::models::APIResponse {
            done: false,
            reason: req.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().json(req.unwrap())
}

/// Adds a bot to the list
#[get("/hiv")]
pub async fn add_bot_api(req: HttpRequest, info: web::Query<UserRequest>, bot: web::Json<CreateBot>) -> HttpResponse {
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
    
    let info = info.into_inner();
    
    let check = sqlx::query!(
        "SELECT api_token FROM users WHERE user_id = $1",
        &info.user_id.to_string()
    )
    .fetch_one(&data.pool)
    .await;

    if check.is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let check = check.unwrap();

    if check.api_token != auth {
        return HttpResponse::Unauthorized().finish();
    }

    let mut bot = bot.into_inner();

    let req = libavacado::bot::add_bot(&data.avacado_public, &data.pool, &info.user_id, &mut bot).await;

    if req.is_err() {
        return HttpResponse::BadRequest().json(crate::models::APIResponse {
            done: false,
            reason: req.unwrap_err().to_string(),
            context: None,
        });
    }

    HttpResponse::Ok().json(crate::models::APIResponse {
        done: true,
        reason: "Added bot".to_string(),
        context: None,
    })
}
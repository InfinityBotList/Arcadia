use actix_web::{get, http::header::HeaderValue, post, web, HttpRequest, HttpResponse};

#[post("/panel/approve")]
pub async fn approve(req: HttpRequest, info: web::Json<crate::models::Request>) -> HttpResponse {
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

#[post("/panel/deny")]
pub async fn deny(req: HttpRequest, info: web::Json<crate::models::Request>) -> HttpResponse {
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

#[post("/panel/votes-reset")]
pub async fn vote_reset(req: HttpRequest, info: web::Json<crate::models::Request>) -> HttpResponse {
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

#[post("/panel/votes-reset/all")]
pub async fn vote_reset_all(req: HttpRequest, info: web::Json<crate::models::GenericRequest>) -> HttpResponse {
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

    HttpResponse::NoContent().body("")
}

/// Get onboarding response data
#[get("/svapi-onboarddata")]
pub async fn staff_verify_onboard_data_api(
    req: HttpRequest,
    q: web::Query<crate::models::SVODQuery>,
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
pub async fn staff_verify_fetch_api(req: HttpRequest, q: web::Query<crate::models::SVQuery>) -> HttpResponse {
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

/// Returns a callback URL for app site
#[get("/herpes/auth")]
pub async fn get_apps_auth_api(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().body(
        format!("https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify", std::env::var("APP_SITE_CLIENT_ID").unwrap(), std::env::var("APP_SITE_REDIRECT_URL").unwrap())
    )
}

/// Performs oauth2 callback for app site
#[get("/herpes/callback")]
pub async fn perform_apps_auth_api(
    req: HttpRequest,
    data: web::Query<crate::models::OauthReq>,
) -> HttpResponse {
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
            reason: "Failed to get access token with status code".to_string()
                + &res.status().to_string(),
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
            reason: "Failed to get user ID with status code".to_string()
                + &res.status().to_string(),
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

    let redirect = format!(
        "https://{}/login/callback?user_id={}&api_token={}",
        data.state, res.id, row.api_token
    );

    HttpResponse::TemporaryRedirect()
        .append_header(("Location", redirect))
        .finish()
}
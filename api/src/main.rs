use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{http, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use libavacado::public::AvacadoPublic;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::{GatewayIntents, Ready};
use slog_scope::info;
use sqlx::postgres::PgPoolOptions;

use dotenv::dotenv;

mod loggy;
mod models;
mod routes;

use crate::models::APIResponse;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

async fn not_found(_req: HttpRequest) -> HttpResponse {
    HttpResponse::build(http::StatusCode::NOT_FOUND).json(models::APIResponse {
        done: false,
        reason: "Not Found".to_string(),
        context: None,
    })
}

fn actix_handle_err<T: std::error::Error + 'static>(err: T) -> actix_web::error::Error {
    let response = HttpResponse::BadRequest().json(APIResponse {
        done: false,
        reason: err.to_string(),
        context: None,
    });
    actix_web::error::InternalError::from_response(err, response).into()
}

struct MainHandler {}

#[async_trait]
impl EventHandler for MainHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Bot is connected!"; "user" => ready.user.name);
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    const MAX_CONNECTIONS: u32 = 3;

    let logger = libteapot::logger::setup_logging("/var/log/arcadia-api.log");

    let _scope_guard = slog_scope::set_global_logger(logger.clone());
    let _log_guard = slog_stdlog::init_with_level(log::Level::Info).unwrap();

    info!("Starting up now!");

    dotenv().ok();

    let pool = PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect(&std::env::var("DATABASE_URL").expect("missing DATABASE_URL"))
        .await
        .expect("Could not initialize connection");

    info!("Connected to postgres/redis"; "pool_size" => pool.size());

    let mut main_cli = serenity::Client::builder(
        std::env::var("DISCORD_TOKEN").expect("No DISCORD_TOKEN specified"),
        GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_PRESENCES,
    )
    .event_handler(MainHandler {})
    .await
    .unwrap();

    let cache_http = Arc::new(main_cli.cache_and_http.clone());

    tokio::task::spawn(async move { main_cli.start().await });

    let app_state = web::Data::new(models::AppState {
        pool,
        cache_http: cache_http.clone(),
        avacado_public: Arc::new(AvacadoPublic::new(
            cache_http.cache.clone(),
            cache_http.http.clone(),
        )),
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| !origin.as_bytes().ends_with(b"bad domain 1"))
            .allowed_methods(vec![
                "GET", "HEAD", "PUT", "POST", "PATCH", "DELETE", "OPTIONS",
            ])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
                http::header::HeaderName::from_bytes(b"SV-Version").unwrap(),
            ])
            .supports_credentials()
            .max_age(1);

        App::new()
            .app_data(app_state.clone())
            .app_data(
                web::JsonConfig::default()
                    .limit(1024 * 1024 * 10)
                    .error_handler(|err, _req| actix_handle_err(err)),
            )
            .app_data(web::QueryConfig::default().error_handler(|err, _req| actix_handle_err(err)))
            .app_data(web::PathConfig::default().error_handler(|err, _req| actix_handle_err(err)))
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .wrap(crate::loggy::Logger)
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::MergeOnly,
            ))
            .default_service(web::route().to(not_found))
            .service(routes::approve)
            .service(routes::deny)
            .service(routes::vote_reset)
            .service(routes::vote_reset_all)
            .service(routes::staff_verify_fetch_api)
            .service(routes::staff_verify_onboard_data_api)
            .service(routes::get_current_maints)
            .service(routes::get_apps_api)
            .service(routes::get_interview_api)
            .service(routes::get_app_list)
            .service(routes::get_apps_auth_api)
            .service(routes::perform_apps_auth_api)
            .service(routes::create_app_api)
            .service(routes::finalize_app_api)
            .service(routes::get_app_api)
            .service(routes::send_interview_api)
    })
    .workers(8)
    .bind("localhost:3010")?
    .run()
    .await
}

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{http, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use log::{debug, error, info};
use sqlx::postgres::PgPoolOptions;

mod models;

use crate::models::APIResponse;

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

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    const MAX_CONNECTIONS: u32 = 3;

    std::env::set_var("RUST_LOG", "api=debug,actix_web=info");
    env_logger::init();
    info!("Starting up...");

    /* We have to create a new AppConfig to get a discord_http client
    This is also negligible cost
    */

    let pool = PgPoolOptions::new()
                        .max_connections(MAX_CONNECTIONS)
                        .connect(&std::env::var("DATABASE_URL").expect("missing DATABASE_URL"))
                        .await
                        .expect("Could not initialize connection");

    debug!("Connected to postgres/redis");

    let app_state = web::Data::new(models::AppState {
        pool,
    });

    error!("This is a error");

    debug!("Connected to redis");

    debug!("Server is starting...");
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| origin.as_bytes().ends_with(b"infinitybots.gg"))
            .allowed_methods(vec![
                "GET", "HEAD", "PUT", "POST", "PATCH", "DELETE", "OPTIONS",
            ])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
                http::header::HeaderName::from_bytes(b"Method").unwrap(),
            ])
            .supports_credentials()
            .max_age(3600);

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
            .wrap(Logger::default())
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::MergeOnly,
            ))
            .default_service(web::route().to(not_found))
    })
    .workers(8)
    .bind("localhost:3010")?
    .run()
    .await
}

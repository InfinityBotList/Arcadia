use std::sync::Arc;
use std::time::Duration;

use actix_cors::Cors;
use actix_web::{http, middleware, web, App, HttpServer};
use libavacado::public::AvacadoPublic;
use log::info;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::{GatewayIntents, Ready};
use sqlx::postgres::PgPoolOptions;

mod models;
mod routes;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

struct MainHandler {}

#[async_trait]
impl EventHandler for MainHandler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Bot is connected: {}", ready.user.name);
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    const MAX_CONNECTIONS: u32 = 3;

    info!("Starting up now!");

    std::env::set_var("RUST_LOG", "api=info");

    env_logger::init();

    let pool = PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect(&libavacado::CONFIG.database_url)
        .await
        .expect("Could not initialize connection");

    info!("Connected to postgres with pool size: {}", pool.size());

    let mut main_cli = serenity::Client::builder(
        &libavacado::CONFIG.token,
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
        ratelimits: moka::future::Cache::builder()
        // Time to live (TTL): 7 minutes
        .time_to_live(Duration::from_secs(60 * 7))
        // Create the cache.
        .build(),        
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().ends_with(libavacado::CONFIG.frontend_url.as_bytes())
                || origin.as_bytes().ends_with("localhost:3000".as_bytes())
            })
            .allowed_methods(vec!["POST", "OPTIONS"])
            .allowed_headers(vec![
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            .max_age(1);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(routes::web_rpc_api)
    })
    .workers(8)
    .bind("localhost:3010")?
    .run()
    .await
}

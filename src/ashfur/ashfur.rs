pub mod authkit;
pub mod config;
pub mod mongoschemas;
pub mod server;

use mongodb::{options::ClientOptions, Client};
use sqlx::{postgres::PgPoolOptions, PgPool};

type Error = Box<dyn std::error::Error + Send + Sync>;

// User data
pub struct Data {
    pub pool: PgPool,
    pub mongo: Client,
}

#[tokio::main]
async fn main() {
    const MAX_CONNECTIONS: u32 = 3; // max connections to the database, we don't need too many here

    std::env::set_var("RUST_LOG", "ashfur=info");

    env_logger::init();

    let mongo_client_options = ClientOptions::parse(config::CONFIG.mongodb_url.clone())
        .await
        .expect("Error parsing MongoDB URL");

    let data = Data {
        pool: PgPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect(&config::CONFIG.database_url)
            .await
            .expect("Could not initialize connection"),
        mongo: Client::with_options(mongo_client_options).expect("Error creating MongoDB client"),
    };

    crate::server::init(data).await;
}

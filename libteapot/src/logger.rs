use std::{fs::OpenOptions, sync::Mutex};

use slog::{o, Drain};

pub fn setup_logging(filename: &'static str) -> slog::Logger {
        // Setup slog
        let file = OpenOptions::new()
        .create(true)
        .append(true)
        .truncate(false)
        .open(filename)
        .unwrap();

    let sqlx_logs = std::env::var("SQLX_LOG").unwrap_or_else(|_| "off".to_string()) == "on";

    let jfile = crate::slogjson_vendored::Json::new(file)
        .add_default_keys()
        .set_flush(false)
        .build()
        .fuse()
        .filter(move |f| {
            // Disable debug logging and spammy stuff
            f.level().is_at_least(slog::Level::Error) 
            || 
                f.level().is_at_least(slog::Level::Info) 
                && !(f.tag() == "tracing::span" || f.tag().starts_with("serenity") || (!sqlx_logs && f.tag().starts_with("sqlx")))
        })
        .fuse();

    let drain = slog_async::Async::new(Mutex::new(jfile).map(slog::Fuse))
        //.overflow_strategy(OverflowStrategy::Block)
        .build()
        .fuse();
    
    let log = slog::Logger::root(drain, o!("version" => env!("CARGO_PKG_VERSION")));

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(35));
        loop {
            interval.tick().await;
            // Check if file is too large (over 10MB)
            if let Ok(metadata) = std::fs::metadata(filename) {
                if metadata.len() > 10_000_000 {
                    // Truncate file
                    let _ = OpenOptions::new().truncate(true).open(filename);
                }
            }
        }
    });

    log
}
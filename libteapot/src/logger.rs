use std::{fs::OpenOptions, sync::Mutex};

use slog::{o, Drain};

pub fn setup_logging(filename: &str) {
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

    let _scope_guard = slog_scope::set_global_logger(log.clone());
    let _log_guard = slog_stdlog::init_with_level(log::Level::Info).unwrap();
}
use std::time::Duration;

pub enum Task {
    Bans,
    AutoUnclaim,
    DeadGuilds,
    StaffResync,
}

pub async fn taskcat(
    pool: sqlx::PgPool,
    cache_http: crate::impls::cache::CacheHttpImpl,
    task: Task,
) -> ! {
    let duration = match task {
        Task::Bans => Duration::from_secs(300),
        Task::AutoUnclaim => Duration::from_secs(60),
        Task::DeadGuilds => Duration::from_secs(60),
        Task::StaffResync => Duration::from_secs(45),
    };

    let task_name = match task {
        Task::Bans => "bans_sync",
        Task::AutoUnclaim => "auto_unclaim",
        Task::DeadGuilds => "dead_guilds",
        Task::StaffResync => "staff_resync",
    };

    let task_desc = match task {
        Task::Bans => "Syncing bans",
        Task::AutoUnclaim => "Checking for claimed bots greater than 1 hour claim interval",
        Task::DeadGuilds => "Checking for dead guilds",
        Task::StaffResync => "Resyncing staff permissions",
    };

    let mut interval = tokio::time::interval(duration);

    loop {
        interval.tick().await;

        log::info!(
            "TASK: {} ({}s interval) [{}]",
            task_name,
            duration.as_secs(),
            task_desc
        );

        if let Err(e) = match task {
            Task::Bans => crate::tasks::bans::bans_sync(&pool, &cache_http).await,
            Task::AutoUnclaim => crate::tasks::autounclaim::auto_unclaim(&pool, &cache_http).await,
            Task::DeadGuilds => crate::tasks::deadguilds::dead_guilds(&pool, &cache_http).await,
            Task::StaffResync => crate::tasks::staffresync::staff_resync(&pool, &cache_http).await,
        } {
            log::error!("TASK {} ERROR'd: {:?}", task_name, e);
        }
    }
}

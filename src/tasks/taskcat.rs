use log::{error, info};
use std::time::Duration;
use strum::{IntoEnumIterator};
use strum_macros::{EnumIter, Display};
use tokio::task::JoinSet;

#[derive(EnumIter, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Task {
    Bans,
    AutoUnclaim,
    StaffResync,
    PremiumRemove,
    SpecRoleSync,
    Uptime,
}

pub async fn start_all_tasks(
    pool: sqlx::PgPool,
    cache_http: crate::impls::cache::CacheHttpImpl,
) -> ! {
    // Start tasks
    let mut set = JoinSet::new();

    for task in Task::iter() {
        set.spawn(crate::tasks::taskcat::taskcat(
            pool.clone(),
            cache_http.clone(),
            task,
        ));
    }

    while let Some(res) = set.join_next().await {
        if let Err(e) = res {
            error!("Error while running task: {}", e);
        }

        info!("Task finished when it shouldn't have");
        std::process::abort();
    }

    info!("All tasks finished when they shouldn't have");
    std::process::abort();
}

async fn taskcat(
    pool: sqlx::PgPool,
    cache_http: crate::impls::cache::CacheHttpImpl,
    task: Task,
) -> ! {
    let duration = match task {
        Task::Bans => Duration::from_secs(300),
        Task::AutoUnclaim => Duration::from_secs(60),
        Task::StaffResync => Duration::from_secs(45),
        Task::PremiumRemove => Duration::from_secs(75),
        Task::SpecRoleSync => Duration::from_secs(50),
        Task::Uptime => Duration::from_secs(90),
    };

    let task_desc = match task {
        Task::Bans => "Syncing bans",
        Task::AutoUnclaim => "Checking for claimed bots greater than 1 hour claim interval",
        Task::StaffResync => "Resyncing staff permissions",
        Task::PremiumRemove => "Removing expired subscriptions",
        Task::SpecRoleSync => "Syncing special roles",
        Task::Uptime => "Uptime Checking",
    };

    let mut interval = tokio::time::interval(duration);

    loop {
        interval.tick().await;

        log::info!(
            "TASK: {} ({}s interval) [{}]",
            task.to_string(),
            duration.as_secs(),
            task_desc
        );

        if let Err(e) = match task {
            Task::Bans => crate::tasks::bans::bans_sync(&pool, &cache_http).await,
            Task::AutoUnclaim => crate::tasks::autounclaim::auto_unclaim(&pool, &cache_http).await,
            Task::StaffResync => crate::tasks::staffresync::staff_resync(&pool, &cache_http).await,
            Task::PremiumRemove => crate::tasks::premium::premium_remove(&pool, &cache_http).await,
            Task::SpecRoleSync => crate::tasks::specrolesync::spec_role_sync(&pool, &cache_http).await,
            Task::Uptime => crate::tasks::uptime::uptime_checker(&pool, &cache_http).await,

        } {
            log::error!("TASK {} ERROR'd: {:?}", task.to_string(), e);
        }
    }
}

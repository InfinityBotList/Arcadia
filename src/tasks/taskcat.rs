use log::{error, info};
use once_cell::sync::Lazy;
use std::time::Duration;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
use tokio::task::JoinSet;
use tokio::sync::Mutex;

static TASK_MUTEX: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));

#[derive(EnumIter, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Task {
    Bans,
    AutoUnclaim,
    StaffResync,
    PremiumRemove,
    SpecRoleSync,
    TeamCleaner,
    GenericCleaner,
    AssetCleaner,
    DeletedBots,
}

impl Task {
    /// Whether or not the task is enabled
    pub fn enabled(&self) -> bool {
        match self {
            Task::Bans => true,
            Task::AutoUnclaim => true,
            Task::StaffResync => true,
            Task::PremiumRemove => true,
            Task::SpecRoleSync => true,
            Task::TeamCleaner => true,
            Task::GenericCleaner => true,
            Task::AssetCleaner => true,
            Task::DeletedBots => true,
        }
    }

    /// How often the task should run
    pub fn duration(&self) -> Duration {
        match self {
            Task::Bans => Duration::from_secs(300),
            Task::AutoUnclaim => Duration::from_secs(60),
            Task::StaffResync => Duration::from_secs(45),
            Task::PremiumRemove => Duration::from_secs(75),
            Task::SpecRoleSync => Duration::from_secs(50),
            Task::TeamCleaner => Duration::from_secs(300),
            Task::GenericCleaner => Duration::from_secs(400),
            Task::AssetCleaner => Duration::from_secs(450),
            Task::DeletedBots => Duration::from_secs(500),
        }
    }

    /// Description of the task
    pub fn description(&self) -> &'static str {
        match self {
            Task::Bans => "Syncing bans",
            Task::AutoUnclaim => "Checking for claimed bots greater than 1 hour claim interval",
            Task::StaffResync => "Resyncing staff permissions",
            Task::PremiumRemove => "Removing expired subscriptions",
            Task::SpecRoleSync => "Syncing special roles",
            Task::TeamCleaner => "Fixing up empty/invalid teams",
            Task::GenericCleaner => "Cleaning up orphaned generic entities",
            Task::AssetCleaner => "Cleaning up orphaned assets",
            Task::DeletedBots => "Cleaning up deleted bots",
        }
    }

    /// Function to run the task
    pub async fn run(
        &self,
        pool: &sqlx::PgPool,
        cache_http: &crate::impls::cache::CacheHttpImpl,
    ) -> Result<(), crate::Error> {
        match self {
            Task::Bans => crate::tasks::bans::bans_sync(pool, cache_http).await,
            Task::AutoUnclaim => crate::tasks::autounclaim::auto_unclaim(pool, cache_http).await,
            Task::StaffResync => crate::tasks::staffresync::staff_resync(pool, cache_http).await,
            Task::PremiumRemove => crate::tasks::premium::premium_remove(pool, cache_http).await,
            Task::SpecRoleSync => {
                crate::tasks::specrolesync::spec_role_sync(pool, cache_http).await
            }
            Task::TeamCleaner => crate::tasks::teamcleaner::team_cleaner(pool).await,
            Task::GenericCleaner => crate::tasks::genericcleaner::generic_cleaner(pool).await,
            Task::AssetCleaner => crate::tasks::assetcleaner::asset_cleaner(pool).await,
            Task::DeletedBots => crate::tasks::deletedbots::deleted_bots(pool, cache_http).await,
        }
    }
}

/// Function to start all tasks
pub async fn start_all_tasks(
    pool: sqlx::PgPool,
    cache_http: crate::impls::cache::CacheHttpImpl,
) -> ! {
    // Start tasks
    let mut set = JoinSet::new();

    for task in Task::iter() {
        if !task.enabled() {
            continue;
        }

        set.spawn(crate::tasks::taskcat::taskcat(
            pool.clone(),
            cache_http.clone(),
            task,
        ));
    }

    if let Some(res) = set.join_next().await {
        if let Err(e) = res {
            error!("Error while running task: {}", e);
        }

        info!("Task finished when it shouldn't have");
        std::process::abort();
    }

    info!("All tasks finished when they shouldn't have");
    std::process::abort();
}

/// Function that manages a task
async fn taskcat(
    pool: sqlx::PgPool,
    cache_http: crate::impls::cache::CacheHttpImpl,
    task: Task,
) -> ! {
    let duration = task.duration();
    let description = task.description();

    // Ensure multiple tx's are not created at the same time
    tokio::time::sleep(duration).await;

    let mut interval = tokio::time::interval(duration);

    loop {
        interval.tick().await;

        let guard = TASK_MUTEX.lock().await;

        log::info!(
            "TASK: {} ({}s interval) [{}]",
            task.to_string(),
            duration.as_secs(),
            description
        );

        if let Err(e) = task.run(&pool, &cache_http).await {
            log::error!("TASK {} ERROR'd: {:?}", task.to_string(), e);
        }

        drop(guard);
    }
}

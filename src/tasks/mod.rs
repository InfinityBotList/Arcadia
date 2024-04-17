pub mod assetcleaner;
pub mod autounclaim;
pub mod bans;
pub mod deletedbots;
pub mod genericcleaner;
pub mod premium;
pub mod specrolesync;
pub mod staffresync;
pub mod teamcleaner;
pub mod voterestter;

use botox::taskman::Task;
use futures_util::FutureExt;

pub fn tasks() -> Vec<Task> {
    vec![
        Task {
            name: "asset_cleaner",
            description: "Cleaning up orphaned assets",
            enabled: true,
            duration: std::time::Duration::from_secs(450),
            run: Box::new(move |ctx| {
                crate::tasks::assetcleaner::asset_cleaner(ctx).boxed()
            }),
        },
        Task {
            name: "auto_unclaim",
            description: "Checking for claimed bots greater than 1 hour claim interval",
            enabled: true,
            duration: std::time::Duration::from_secs(60),
            run: Box::new(move |ctx| {
                crate::tasks::autounclaim::auto_unclaim(ctx).boxed()
            }),
        },
        Task {
            name: "bans_sync",
            description: "Syncing bans",
            enabled: true,
            duration: std::time::Duration::from_secs(300),
            run: Box::new(move |ctx| {
                crate::tasks::bans::bans_sync(ctx).boxed()
            }),
        },
        Task {
            name: "deleted_bots",
            description: "Cleaning up deleted bots",
            enabled: true,
            duration: std::time::Duration::from_secs(500),
            run: Box::new(move |ctx| {
                crate::tasks::deletedbots::deleted_bots(ctx).boxed()
            }),
        },
        Task {
            name: "generic_cleaner",
            description: "Cleaning up orphaned generic entities",
            enabled: true,
            duration: std::time::Duration::from_secs(400),
            run: Box::new(move |ctx| {
                crate::tasks::genericcleaner::generic_cleaner(ctx).boxed()
            }),
        },
        Task {
            name: "premium_remove",
            description: "Removing expired subscriptions",
            enabled: true,
            duration: std::time::Duration::from_secs(75),
            run: Box::new(move |ctx| {
                crate::tasks::premium::premium_remove(ctx).boxed()
            }),
        },
        Task {
            name: "spec_role_sync",
            description: "Syncing special roles",
            enabled: true,
            duration: std::time::Duration::from_secs(50),
            run: Box::new(move |ctx| {
                crate::tasks::specrolesync::spec_role_sync(ctx).boxed()
            }),
        },
        Task {
            name: "staff_resync",
            description: "Resyncing staff permissions",
            enabled: true,
            duration: std::time::Duration::from_secs(45),
            run: Box::new(move |ctx| {
                crate::tasks::staffresync::staff_resync(ctx).boxed()
            }),
        },
        Task {
            name: "team_cleaner",
            description: "Fixing up empty/invalid teams",
            enabled: true,
            duration: std::time::Duration::from_secs(300),
            run: Box::new(move |ctx| {
                crate::tasks::teamcleaner::team_cleaner(ctx).boxed()
            }),
        },
        Task {
            name: "vote_resetter",
            description: "Resetting votes",
            enabled: true,
            duration: std::time::Duration::from_secs(600),
            run: Box::new(move |ctx| {
                crate::tasks::voterestter::vote_resetter(ctx).boxed()
            }),
        },
    ]
}
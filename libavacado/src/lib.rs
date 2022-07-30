pub mod checks;
pub mod staff;
pub mod types;
pub mod manage;
pub mod public;
pub mod search;
pub mod maps;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
pub const GIT_SEMVER: &str = env!("VERGEN_GIT_SEMVER");
pub const GIT_COMMIT_MSG: &str = env!("VERGEN_GIT_COMMIT_MESSAGE");
pub const BUILD_CPU: &str = env!("VERGEN_SYSINFO_CPU_BRAND");
pub const CARGO_PROFILE: &str = env!("VERGEN_CARGO_PROFILE");
pub const RUSTC_VERSION: &str = env!("VERGEN_RUSTC_SEMVER");
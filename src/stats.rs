use poise::serenity_prelude::{CreateEmbed, Color};
use poise::CreateReply;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

// Various statistics
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
pub const GIT_SEMVER: &str = env!("VERGEN_GIT_SEMVER");
pub const GIT_COMMIT_MSG: &str = env!("VERGEN_GIT_COMMIT_MESSAGE");
pub const BUILD_CPU: &str = env!("VERGEN_SYSINFO_CPU_BRAND");
pub const CARGO_PROFILE: &str = env!("VERGEN_CARGO_PROFILE");
pub const RUSTC_VERSION: &str = env!("VERGEN_RUSTC_SEMVER");

#[poise::command(category = "Stats", prefix_command, slash_command, user_cooldown = 1)]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    let msg = CreateReply::default().embed(
        CreateEmbed::default()
            .title("Bot Information:")
            .color(Color::from_rgb(0, 255, 0))
            .field("Bot Version:", VERSION, true)
            .field("RustC Version:", RUSTC_VERSION, true)
            .field(
                "Git Commit:",
                GIT_SHA.to_string() + "(semver=" + GIT_SEMVER + ")",
                true,
            )
            .field("Commit Message:", GIT_COMMIT_MSG, true)
            .field("Built On:", BUILD_CPU, true)
            .field("Cargo Profile:", CARGO_PROFILE, true),
    );

    ctx.send(msg).await?;
    Ok(())
}

/// Look at our site analytics!
#[poise::command(category = "Stats", slash_command, prefix_command)]
pub async fn analytics(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    let categorizedbots = sqlx::query!("SELECT type as method, COUNT(*) FROM bots GROUP BY type;")
        .fetch_all(&data.pool)
        .await?;

    let bots = sqlx::query!("SELECT COUNT(*) FROM bots;")
        .fetch_one(&data.pool)
        .await?;

    let teams = sqlx::query!("SELECT COUNT(*) FROM teams;")
        .fetch_one(&data.pool)
        .await?;

    let users = sqlx::query!("SELECT COUNT(*) FROM users;")
        .fetch_one(&data.pool)
        .await?;

    let guilds = sqlx::query!("SELECT COUNT(*) FROM servers;")
        .fetch_one(&data.pool)
        .await?;

    let mut approved = 0;
    let mut denied = 0;
    let mut certified = 0;
    let mut testbot = 0;
    for stat in categorizedbots {
        if stat.method == "approved" {
            approved = stat.count.unwrap_or_default();
        }
        if stat.method == "denied" {
            denied = stat.count.unwrap_or_default();
        }
        if stat.method == "certified" {
            certified = stat.count.unwrap_or_default();
        }
        if stat.method == "testbot" {
            testbot = stat.count.unwrap_or_default();
        }
    }

    let embed = CreateEmbed::default()
        .title("Infinity List Analytics")
        .description("I hope it's good :eyes:")
        .color(Color::from_rgb(0, 255, 0))
        .field(
            "User Count:",
            users.count.unwrap_or_default().to_string(),
            true,
        )
        .field(
            "Team Count:",
            teams.count.unwrap_or_default().to_string(),
            true,
        )
        .field(
            "Bot Count:",
            bots.count.unwrap_or_default().to_string(),
            true,
        )
        .field(
            "Server Count:",
            guilds.count.unwrap_or_default().to_string(),
            true,
        )
        .field("Approved Bots:", approved.to_string(), true)
        .field("Denied Bots:", denied.to_string(), true)
        .field("Certified Bots:", certified.to_string(), true)
        .field("Test Bots (hidden):", testbot.to_string(), true);

    let msg = CreateReply::default().embed(embed);
    ctx.send(msg).await?;
    Ok(())
}

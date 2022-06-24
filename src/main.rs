use poise::serenity_prelude as serenity;
use dotenv::dotenv;
use log::{error, info};
use sqlx::postgres::PgPoolOptions;
use std::fmt::Write as _; // import without risk of name clashing
use serenity::id::UserId;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
// User data, which is stored and accessible in all command invocations
pub struct Data {
    pool: sqlx::PgPool,
}

/// Check for staff_server
async fn staff_server(ctx: Context<'_>) -> Result<bool, Error> {
    let in_staff_server = match ctx.guild_id() {
        Some(guild_id) => {
            guild_id.0.to_string() == std::env::var("STAFF_SERVER")?
        }
        None => false,
    };
    
    Ok(in_staff_server)
}

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(prefix_command)]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            error!("Error in command `{}`: {:?}", ctx.command().name, error,);
            ctx.say(format!(
                "There was an error running this command: {:?}",
                error
            ))
            .await
            .unwrap();
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e);
            }
        }
    }
}

#[poise::command(track_edits, prefix_command, slash_command)]
async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            show_context_menu_commands: true,
            ..poise::builtins::HelpConfiguration::default()
        },
    )
    .await?;
    Ok(())
}

#[poise::command(track_edits, prefix_command, slash_command, check = "staff_server")]
async fn staff(ctx: Context<'_>) -> Result<(), Error> {
    // Get list of users with staff flag set to true
    let data = ctx.data();

    let staffs = sqlx::query!(
        "SELECT user_id, username FROM users WHERE staff = true ORDER BY user_id ASC"
    )
    .fetch_all(&data.pool)
    .await?;

    let mut staff_list = "**Staff List**\n".to_string();
    let mut not_in_staff_server = "**Not in staff server (based on cache, may be inaccurate)**\n".to_string();

    let guild = ctx.guild().unwrap();

    for staff in staffs.iter() {
        // Convert ID to u64
        let user_id = staff.user_id.parse::<u64>()?;

        let cache_user = ctx.discord().cache.member(guild.id, UserId(user_id));

        let user = match cache_user {
            Some(user) => user.user,
            None => {
                // User not found in cache, fetch from API
                let user = UserId(user_id).to_user(&ctx.discord().http).await?;

                write!(not_in_staff_server, "{} ({})", user.id.0, user.name)?;
                user
            }
        };

        writeln!(staff_list, "{} ({})", staff.user_id, user.name)?;
    }

    ctx.say(staff_list + "\n" + &not_in_staff_server).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    const MAX_CONNECTIONS: u32 = 3; // max connections to the database, we don't need too many here

    std::env::set_var("RUST_LOG", "sovngarde=debug");
    env_logger::init();
    info!("Starting Sovngarde...");

    dotenv().ok();

    let framework = poise::Framework::build()
        .options(poise::FrameworkOptions {
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("ibb!".into()),
                ..poise::PrefixFrameworkOptions::default()
            },
            commands: vec![
                age(), 
                register(),
                help(),
                staff()
            ],
            /// This code is run before every command
            pre_command: |ctx| {
                Box::pin(async move {
                    info!(
                        "Executing command {} for user {} ({})...",
                        ctx.command().qualified_name,
                        ctx.author().name,
                        ctx.author().id
                    );
                })
            },
            /// This code is run after every command returns Ok
            post_command: |ctx| {
                Box::pin(async move {
                    info!(
                        "Done executing command {} for user {} ({})...",
                        ctx.command().qualified_name,
                        ctx.author().name,
                        ctx.author().id
                    );
                })
            },
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::all())
        .user_data_setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    pool: PgPoolOptions::new()
                        .max_connections(MAX_CONNECTIONS)
                        .connect(&std::env::var("DATABASE_URL").expect("missing DATABASE_URL"))
                        .await
                        .expect("Could not initialize connection"),
                })
            })
        });

    framework.run().await.expect("Error");
}

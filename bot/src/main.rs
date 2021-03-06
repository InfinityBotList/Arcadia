use std::{sync::Arc, time::Duration};
use std::fmt::Write;

use dotenv::dotenv;
use log::{error, info};
use poise::serenity_prelude::{self as serenity, GuildId};
use sqlx::postgres::PgPoolOptions;

use poise::serenity_prelude::{ChannelId, UserId};

use poise::Command;

mod _checks;
mod _onboarding;
mod _utils;
mod admin;
mod staff;
mod testing;
mod search;
mod tests;
mod stats;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
// User data, which is stored and accessible in all command invocations
pub struct Data {
    pool: sqlx::PgPool,
    avacado_public: libavacado::public::AvacadoPublic
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
                "There was an error running this command: {}",
                error
            ))
            .await
            .unwrap();
        },
        poise::FrameworkError::CommandCheckFailed { error, ctx } => {
            error!("Error in command `{}`: {:?}", ctx.command().name, error,);
            if let Some(error) = error {
                ctx.say(format!(
                    "Whoa there, do you have permission to do this?: {}",
                    error
                ))
                .await
                .unwrap();
            } else {
                ctx.say("You don't have permission to do this but we couldn't figure out why...").await.unwrap();
            }
        },
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

async fn _embed_help(ctx: poise::FrameworkContext<'_, Data, Error>, page: u32) -> Result<String, Error> {
    let mut categories = libavacado::maps::OrderedMap::<Option<&str>, Vec<&Command<Data, Error>>>::new();
    for cmd in &ctx.options().commands {
        categories
            .get_or_insert_with(cmd.category, Vec::new)
            .push(cmd);
    }

    let mut menu = format!("**Page:** {}", page);
    for (category_name, commands) in categories {
        menu += category_name.unwrap_or("Commands");
        menu += ":\n";
        for command in commands {
            if command.hide_in_help {
                continue;
            }

            let _ = writeln!(
                menu,
                "/{cmd_name} | ibb!{cmd_name} - {desc}",
                cmd_name=command.name,
                desc=command.description.as_deref().unwrap_or("")
            );
        }
    }

    Ok(menu)
} 

#[poise::command(track_edits, prefix_command, slash_command)]
async fn new_help(
    ctx: Context<'_>,
) -> Result<(), Error> {
    _embed_help(ctx.framework(), 1).await?;

    Ok(())
}

async fn event_listener(
    ctx: &serenity::Context,
    event: &poise::Event<'_>,
    user_data: &Data,
) -> Result<(), Error> {
    let main_server = std::env::var("MAIN_SERVER")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let testing_server = std::env::var("TESTING_SERVER")
        .unwrap()
        .parse::<u64>()
        .unwrap();

    match event {
        poise::Event::Ready { data_about_bot } => {
            info!(
                "{} is ready! Doing some minor DB fixes",
                data_about_bot.user.name
            );
            sqlx::query!(
                "UPDATE bots SET claimed_by = NULL, claimed = false WHERE LOWER(claimed_by) = 'none'",
            )
            .execute(&user_data.pool)
            .await?;

            let _ctx = ctx.to_owned();
            let pool = user_data.pool.clone();

            tokio::task::spawn(async move {
                autounclaim(pool, _ctx.http, _ctx.cache).await;
            });
        }
        poise::Event::CacheReady { guilds } => {
            info!("Cache ready with {} guilds", guilds.len());
        }
        poise::Event::GuildMemberAddition { new_member } => {
            if new_member.guild_id.0 == main_server && new_member.user.bot {
                // Check if new memebr is in testing server
                let member =
                    ctx.cache
                        .member_field(GuildId(testing_server), new_member.user.id, |m| m.user.id);

                if member.is_some() {
                    // If so, move them to main server
                    GuildId(testing_server)
                        .kick_with_reason(&ctx, new_member.user.id, "Added to main server")
                        .await?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

async fn autounclaim(
    pool: sqlx::PgPool,
    http: Arc<serenity::http::Http>,
    cache: Arc<serenity::Cache>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(10000));

    let lounge_channel_id = ChannelId(
        std::env::var("LOUNGE_CHANNEL")
            .unwrap()
            .parse::<u64>()
            .unwrap(),
    );

    let main_server = std::env::var("MAIN_SERVER")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let staff_server = std::env::var("STAFF_SERVER")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let testing_server = std::env::var("TESTING_SERVER")
        .unwrap()
        .parse::<u64>()
        .unwrap();

    loop {
        interval.tick().await;
        info!("Checking for claimed bots greater than 1 hour claim interval");

        let res = sqlx::query!(
            "SELECT bot_id, claimed_by, last_claimed, owner FROM bots WHERE claimed = true AND NOW() - last_claimed > INTERVAL '1 hour'",
        )
        .fetch_all(&pool)
        .await;

        if res.is_err() {
            error!(
                "Error while checking for claimed bots: {:?}",
                res.unwrap_err()
            );
            continue;
        }

        let bots = res.unwrap();

        for bot in bots {
            if bot.claimed_by.is_none() {
                info!(
                    "Unclaiming bot {} because it has no staff who has claimed it",
                    bot.bot_id
                );
                let res = sqlx::query!(
                    "UPDATE bots SET claimed_by = NULL, claimed = false WHERE bot_id = $1",
                    bot.bot_id
                )
                .execute(&pool)
                .await;

                if res.is_err() {
                    error!(
                        "Error while unclaiming bot {}: {:?}",
                        bot.bot_id,
                        res.unwrap_err()
                    );
                    continue;
                }

                continue;
            }

            if bot.last_claimed.is_none() {
                info!(
                    "Unclaiming bot {} because it has no last_claimed time",
                    bot.bot_id
                );
                let res = sqlx::query!(
                    "UPDATE bots SET claimed_by = NULL, claimed = false WHERE bot_id = $1",
                    bot.bot_id
                )
                .execute(&pool)
                .await;

                if res.is_err() {
                    error!(
                        "Error while unclaiming bot {}: {:?}",
                        bot.bot_id,
                        res.unwrap_err()
                    );
                    continue;
                }

                continue;
            }

            let claimed_by = bot.claimed_by.unwrap();
            let last_claimed = bot.last_claimed.unwrap();

            info!(
                "Unclaiming bot {} because it was claimed by {} and never unclaimed",
                bot.bot_id, claimed_by
            );
            let res = sqlx::query!(
                "UPDATE bots SET claimed_by = NULL, claimed = false WHERE bot_id = $1",
                bot.bot_id
            )
            .execute(&pool)
            .await;

            if res.is_err() {
                error!(
                    "Error while unclaiming bot {}: {:?}",
                    bot.bot_id,
                    res.unwrap_err()
                );
                continue;
            }

            let start_time = chrono::offset::Utc::now();

            // Now send message in #lounge
            let err = lounge_channel_id.send_message(&http, |m| {
                m.content(format!("<@{}>", claimed_by))
                .embed(|e| {
                    e.title("Auto-Unclaimed Bot")
                        .description(
                            format!(
                                "Bot <@{}> was auto-unclaimed (was previously claimed by <@{}> due to it being claimed for over one hour without being approved or denied).\nThis bot was last claimed at {} ({}).", 
                                bot.bot_id,
                                claimed_by,
                                last_claimed.format("%Y-%m-%d %H:%M:%S"),
                                (start_time - last_claimed).num_minutes().to_string() + " minutes ago"
                            ))
                        .color(0xFF0000)
                    })
            })
            .await;

            if err.is_err() {
                error!(
                    "Error while sending message to lounge: {:?}",
                    err.unwrap_err()
                );
                continue;
            }

            let owner = bot.owner.parse::<u64>();

            if let Ok(owner) = owner {
                let private_channel = UserId(owner).create_dm_channel(&http).await;

                if private_channel.is_err() {
                    error!(
                        "Error while sending message to owner: {:?}",
                        private_channel.unwrap_err()
                    );
                    continue;
                }

                let private_channel = private_channel.unwrap();

                let err = private_channel.send_message(&http, |m| {
                    m.embed(|e| {
                        e.title("Bot Unclaimed!");
                        e.description(
                            format!(
                                r#"
<@{}> has been unclaimed as it was not being actively reviewed. 

Don't worry, this is normal, could just be our staff looking more into your bots functionality! 

For more information, you can contact the current reviewer <@{}>

*This bot was claimed at {} ({}). This is a automated message letting you know about whats going on...*
                                "#, 
                                bot.bot_id,
                                claimed_by,
                                last_claimed.format("%Y-%m-%d %H:%M:%S"),
                                (start_time - last_claimed).num_minutes().to_string() + " minutes ago"
                            ));
                        e.footer(|f| {
                            f.text("This is completely normal, don't worry!");
                            f
                        });
                        e
                    });
                    m
                })
                .await;

                if err.is_err() {
                    error!(
                        "Error while sending message to owner: {:?}",
                        err.unwrap_err()
                    );
                    continue;
                }
            }
        }

        info!("Checking for dead guilds made by staff");

        // Loop through all guilds (more optimized that a normal postgres check)
        let guilds = cache.guilds();

        for guild_id in guilds {
            // Check if guild is official (main/testing/staff)
            if guild_id.0 == main_server
                || guild_id.0 == testing_server
                || guild_id.0 == staff_server
            {
                continue;
            }
            // Get guild name from cache
            let guild = guild_id.name(&cache);

            if guild.is_none() {
                error!("Error while getting guild name with ID: {:?}", guild_id);
                continue;
            }

            // Try parsing guild name to u64
            let guild_u64 = guild.as_ref().unwrap().parse::<u64>();

            if guild_u64.is_err() {
                // We have a bad guild name, delete or leave if we are not owner
                info!(
                    "Deleting guild {} because it is not a valid guild name: {}",
                    guild_id,
                    guild.unwrap()
                );
                _utils::delete_leave_guild(&http, &cache, guild_id).await;
            } else {
                // Check that this guild_u64 is in database under users AND that it is not dead
                let res = sqlx::query!(
                    "SELECT user_id FROM users WHERE user_id = $1 AND NOW() - staff_onboard_last_start_time < interval '1 hour' AND NOT(staff_onboard_state = 'complete' OR staff_onboard_state = 'pending-manager-review')",
                    guild_u64.unwrap().to_string()
                )
                .fetch_one(&pool)
                .await;

                if res.is_err() {
                    match res.as_ref().unwrap_err() {
                        sqlx::Error::RowNotFound => {
                            _utils::delete_leave_guild(&http, &cache, guild_id).await;
                        }
                        _ => {
                            error!(
                                "Error while checking if guild is in database: {:?}",
                                res.unwrap_err()
                            );
                            continue;
                        }
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    const MAX_CONNECTIONS: u32 = 3; // max connections to the database, we don't need too many here

    std::env::set_var("RUST_LOG", "bot=debug");
    env_logger::init();
    info!("Starting Arcadia (bot)...");

    dotenv().ok();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("ibb!".into()),
                ..poise::PrefixFrameworkOptions::default()
            },
            listener: |ctx, event, _framework, user_data| {
                Box::pin(event_listener(ctx, event, user_data))
            },
            commands: vec![
                age(),
                register(),
                help(),
                staff::staff(),
                testing::onboard(),
                testing::invite(),
                testing::claim(),
                testing::unclaim(),
                testing::queue(),
                testing::approve(),
                testing::deny(),
                testing::staffguide(),
                tests::test_staffcheck(),
                tests::test_admin_dev(),
                tests::test_admin(),
                admin::update_field(),
                admin::votereset(),
                admin::voteresetall(),
                admin::approveonboard(),
                search::searchbots(),
                stats::stats(),
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
                    // Some onboarding things need a post command to be executed
                    let res = crate::_onboarding::post_command(ctx).await;

                    if let Err(e) = res {
                        error!("Error while executing onboarding post command: {:?}", e);
                        if let Err(discord_err) = ctx
                            .say(
                                "Onboarding background daemon failed with error: ".to_string()
                                    + e.to_string().as_str(),
                            )
                            .await
                        {
                            error!("Error while sending message to user: {:?}", discord_err);
                        }
                    }

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
                    avacado_public: libavacado::public::AvacadoPublic::new(_ctx.cache.clone(), _ctx.http.clone())
                })
            })
        });

    framework.run().await.expect("Error");
}

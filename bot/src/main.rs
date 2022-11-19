use std::{sync::Arc, time::Duration};

use dotenv::dotenv;
use log::{error, info};
use poise::serenity_prelude::{self as serenity, GuildId};
use sqlx::postgres::PgPoolOptions;

use poise::serenity_prelude::{ChannelId, UserId};

mod _checks;
mod _onboarding;
mod _utils;
mod admin;
mod botowners;
mod explain;
mod help;
mod search;
mod staff;
mod stats;
mod testing;
mod tests;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
// User data, which is stored and accessible in all command invocations
pub struct Data {
    pool: sqlx::PgPool,
    avacado_public: libavacado::public::AvacadoPublic,
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

/// Test followup
#[poise::command(slash_command, prefix_command)]
async fn actf(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("initial response").await?;
    ctx.say("followup").await?;

    Ok(())
}

/// Test await_component_interaction
#[poise::command(slash_command, prefix_command)]
async fn act(ctx: Context<'_>) -> Result<(), Error> {
    let msg = ctx
        .send(|m| {
            m.content("Test").components(|c| {
                c.create_action_row(|f| {
                    f.create_button(|b| {
                        b.label("A")
                            .custom_id("abc")
                            .style(serenity::ButtonStyle::Danger)
                    })
                })
            })
        })
        .await?
        .into_message()
        .await?;

    let interaction = msg
        .await_component_interaction(ctx.discord())
        .author_id(ctx.author().id)
        .await;

    if let Some(m) = &interaction {
        let id = &m.data.custom_id;
        info!("Received interaction: {}", id);
        ctx.say(format!("Received interaction: {}", id)).await?;
    } else {
        info!("No interaction");
        ctx.say("No interaction received").await?;
    }

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
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            error!("Error in command `{}`: {:?}", ctx.command().name, error,);
            ctx.say(format!(
                "There was an error running this command: {}",
                error
            ))
            .await
            .unwrap();
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx } => {
            error!(
                "[Possible] error in command `{}`: {:?}",
                ctx.command().name,
                error,
            );
            if let Some(error) = error {
                error!("Error in command `{}`: {:?}", ctx.command().name, error,);
                ctx.say(format!(
                    "Whoa there, do you have permission to do this?: {}",
                    error
                ))
                .await
                .unwrap();
            } else {
                ctx.say("You don't have permission to do this but we couldn't figure out why...")
                    .await
                    .unwrap();
            }
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e);
            }
        }
    }
}

#[poise::command(track_edits, prefix_command, slash_command)]
async fn simplehelp(
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
        poise::Event::InteractionCreate { interaction } => {
            info!("Interaction received: {:?}", interaction.id());
        }
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

            let autounclaim_events =
                std::env::var("AUTOUNCLAIM_EVENTS").unwrap_or_else(|_| "true".to_string());

            if autounclaim_events == "true" {
                tokio::task::spawn(async move {
                    autounclaim(pool, _ctx.http, _ctx.cache).await;
                });
            }
        }
        poise::Event::CacheReady { guilds } => {
            info!("Cache ready with {} guilds", guilds.len());
        }
        poise::Event::GuildMemberAddition { new_member } => {
            if new_member.guild_id.0 == main_server && new_member.user.bot {
                // Check if new member is in testing server
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
    let mut interval = tokio::time::interval(Duration::from_millis(30000));

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

    let logger = libteapot::logger::setup_logging("/var/log/arcadia-bot.log");

    let _scope_guard = slog_scope::set_global_logger(logger.clone());
    let _log_guard = slog_stdlog::init_with_level(log::Level::Info).unwrap();

    info!("Starting Arcadia (bot)...");

    dotenv().ok();

    // proxy url is always http://localhost:3219
    let mut proxy_url = "http://localhost:3219".to_string();
    if let Ok(v) = std::env::var("PROXY_URL") {
        info!("Setting proxy url to {}", v);
        proxy_url = v;
    }

    info!("Proxy URL: {}", proxy_url);

    // http_pre is for getting app_info etc., http is for poise framework
    let http_pre =
        serenity::HttpBuilder::new(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
            .proxy(&proxy_url)
            .expect("proxy error")
            .ratelimiter_disabled(true)
            .build();
    let http =
        serenity::HttpBuilder::new(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
            .proxy(proxy_url)
            .expect("proxy error")
            .ratelimiter_disabled(true)
            .build();

    let client_builder =
        serenity::ClientBuilder::new_with_http(http, serenity::GatewayIntents::all());

    // Get the bot's owners and id and convert it to hashset
    let app_inf = http_pre.get_current_application_info().await.unwrap();
    let owners = app_inf
        .team
        .as_ref()
        .map(|team| team.members.iter().map(|m| m.user.id).collect())
        .unwrap_or_else(|| vec![app_inf.owner.id]);
    let owners = owners.into_iter().collect::<std::collections::HashSet<_>>();

    let framework = poise::Framework::new(
        client_builder,
        move |_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    pool: PgPoolOptions::new()
                        .max_connections(MAX_CONNECTIONS)
                        .connect(&std::env::var("DATABASE_URL").expect("missing DATABASE_URL"))
                        .await
                        .expect("Could not initialize connection"),
                    avacado_public: libavacado::public::AvacadoPublic::new(
                        _ctx.cache.clone(),
                        _ctx.http.clone(),
                    ),
                })
            })
        },
        poise::FrameworkOptions {
            owners,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("ibb!".into()),
                ..poise::PrefixFrameworkOptions::default()
            },
            listener: |ctx, event, _framework, user_data| {
                Box::pin(event_listener(ctx, event, user_data))
            },
            commands: vec![
                age(),
                act(),
                actf(),
                register(),
                simplehelp(),
                help::help(),
                explain::explainme(),
                staff::staff(),
                testing::onboard(),
                testing::invite(),
                testing::claim(),
                testing::claim_slash(),
                testing::claim_context(),
                testing::unclaim(),
                testing::unclaim_context(),
                testing::queue(),
                testing::approve(),
                testing::deny(),
                testing::staffguide(),
                tests::test_staffcheck(),
                tests::test_admin_dev(),
                tests::test_admin(),
                tests::test_poll(),
                admin::update_field(),
                admin::votereset(),
                admin::voteresetall(),
                admin::onboardman(),
                search::searchbots(),
                stats::stats(),
                botowners::setstats(),
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
        },
    )
    .await
    .expect("Error");

    framework.start().await.expect("Error");
}

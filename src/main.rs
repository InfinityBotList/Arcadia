use log::{error, info};
use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, CreateMessage, FullEvent, Timestamp},
    CreateReply,
};
use sqlx::postgres::PgPoolOptions;

use botox::cache::CacheHttpImpl;
use std::sync::Arc;

mod botowners;
mod checks;
mod config;
mod explain;
mod help;
mod impls;
mod panelapi;
mod rpc;
mod rpc_command;
mod staff;
mod stats;
mod tasks;
mod test;
mod testing;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
// User data, which is stored and accessible in all command invocations
pub struct Data {
    pool: sqlx::PgPool,
}

/// Look at our site analytics!
#[poise::command(category = "Stats", slash_command, prefix_command)]
async fn analytics(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    let categorizedbots = sqlx::query!("SELECT type as method, COUNT(*) FROM bots GROUP BY type;")
        .fetch_all(&data.pool)
        .await?;

    let bots = sqlx::query!("SELECT COUNT(*) FROM bots;")
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
    }

    let embed = CreateEmbed::default()
        .title("Infinity List Analytics")
        .description("I hope it's good :eyes:")
        .field(
            "User Count:",
            users.count.unwrap_or_default().to_string(),
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
        .field("Certified Bots:", certified.to_string(), true);

    let msg = CreateReply::default().embed(embed);
    ctx.send(msg).await?;
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
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!("Error in command `{}`: {:?}", ctx.command().name, error,);
            let err = ctx
                .say(format!(
                    "There was an error running this command: {}",
                    error
                ))
                .await;

            if let Err(e) = err {
                error!("SQLX Error: {}", e);
            }
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            error!(
                "[Possible] error in command `{}`: {:?}",
                ctx.command().name,
                error,
            );
            if let Some(error) = error {
                error!("Error in command `{}`: {:?}", ctx.command().name, error,);
                let err = ctx
                    .say(format!(
                        "Whoa there, do you have permission to do this?: {}",
                        error
                    ))
                    .await;

                if let Err(e) = err {
                    error!("Error while sending error message: {}", e);
                }
            } else {
                let err = ctx
                    .say("You don't have permission to do this but we couldn't figure out why...")
                    .await;

                if let Err(e) = err {
                    error!("Error while sending error message: {}", e);
                }
            }
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e);
            }
        }
    }
}

async fn event_listener<'a>(
    ctx: poise::FrameworkContext<'a, Data, Error>,
    event: &FullEvent,
) -> Result<(), Error> {
    let user_data = ctx.serenity_context.data::<Data>();
    match event {
        FullEvent::InteractionCreate { interaction } => {
            info!("Interaction received: {:?}", interaction.id());
        }
        FullEvent::CacheReady { guilds } => {
            info!("Cache ready with {} guilds", guilds.len());
        }
        FullEvent::Ready { data_about_bot } => {
            info!(
                "{} is ready! Doing some minor DB fixes",
                data_about_bot.user.name
            );

            sqlx::query!(
                "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE LOWER(claimed_by) = 'none'",
            )
            .execute(&user_data.pool)
            .await?;

            // Start RPC
            let cache_http_papi = CacheHttpImpl {
                http: ctx.serenity_context.http.clone(),
                cache: ctx.serenity_context.cache.clone(),
            };

            tokio::task::spawn(panelapi::server::init_panelapi(
                user_data.pool.clone(),
                cache_http_papi,
            ));

            if *crate::config::CURRENT_ENV != "staging" {
                tokio::task::spawn(botox::taskman::start_all_tasks(
                    crate::tasks::tasks(),
                    ctx.serenity_context.clone(),
                ));
            }
        }
        FullEvent::GuildMemberAddition { new_member } => {
            if *crate::config::CURRENT_ENV == "staging" {
                return Ok(());
            }

            if new_member.guild_id == config::CONFIG.servers.main && new_member.user.bot() {
                // Send member join message
                config::CONFIG.channels.system
                .send_message(
                    &ctx.serenity_context.http,
                    CreateMessage::new()
                    .embed(
                        CreateEmbed::default()
                        .title("__**New Bot Added**__")
                        .description(
                            format!(
                                "Bot <@{}> ({}) has joined the server and has been given the `Bots` role.",
                                new_member.user.id,
                                new_member.user.name
                            )
                        )
                        .color(0x00ff00)
                        .thumbnail(new_member.user.face())
                        .timestamp(Timestamp::now())
                    )
                )
                .await?;

                // Give bot role
                ctx.serenity_context
                    .http
                    .add_member_role(
                        config::CONFIG.servers.main,
                        new_member.user.id,
                        config::CONFIG.roles.bot_role,
                        Some("Bot added to server"),
                    )
                    .await?;
            }

            if new_member.guild_id == config::CONFIG.servers.main && !new_member.user.bot() {
                // Send member join message
                config::CONFIG.channels.system
                .send_message(
                    &ctx.serenity_context.http,
                    CreateMessage::new()
                    .embed(
                        CreateEmbed::default()
                        .title("__**New User**__")
                        .description(
                            format!(
                                "Hmmmm... looks like <@{}> ({}) has strolled in. Can we trust them?",
                                new_member.user.id,
                                new_member.user.name
                            )
                        )
                        .color(0x00ff00)
                        .thumbnail(new_member.user.face())
                        .timestamp(Timestamp::now())
                    )
                )
                .await?;
            }
        }
        _ => {}
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    const MAX_CONNECTIONS: u32 = 6; // max connections to the database, we don't need too many here

    std::env::set_var("RUST_LOG", "bot=info, moka=error");

    env_logger::init();

    info!("Proxy URL: {}", config::CONFIG.proxy_url);

    let http = Arc::new(
        serenity::HttpBuilder::new(&config::CONFIG.token.get())
            .proxy(config::CONFIG.proxy_url.clone())
            .ratelimiter_disabled(true)
            .build(),
    );

    let client_builder =
        serenity::ClientBuilder::new_with_http(http, serenity::GatewayIntents::all());

    let data = Data {
        pool: PgPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect(&config::CONFIG.database_url)
            .await
            .expect("Could not initialize connection"),
    };

    let prefix = crate::config::CONFIG.prefix.get();

    let framework = poise::Framework::new(poise::FrameworkOptions {
        initialize_owners: true,
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(prefix.into()),
            ..poise::PrefixFrameworkOptions::default()
        },
        event_handler: |ctx, event| Box::pin(event_listener(ctx, event)),
        commands: vec![
            analytics(),
            register(),
            help::help(),
            explain::explainme(),
            staff::staff(),
            testing::invite(),
            testing::invitesafe(),
            testing::claim(),
            testing::unclaim(),
            testing::queue(),
            testing::approve(),
            testing::deny(),
            testing::staffguide(),
            stats::stats(),
            botowners::getbotroles(),
            rpc_command::rpc(),
            rpc_command::rpclist(),
        ],
        // This code is run before every command
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
        // This code is run after every command returns Ok
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
    });

    let mut client = client_builder
        .framework(framework)
        .data(Arc::new(data))
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

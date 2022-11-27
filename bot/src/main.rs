use std::{num::NonZeroU64, time::Duration};

use dotenv::dotenv;
use log::{error, info};
use poise::serenity_prelude::{
    self as serenity, CreateEmbed, CreateEmbedFooter, CreateMessage, FullEvent, GuildId,
};
use sqlx::postgres::PgPoolOptions;

use poise::serenity_prelude::{ChannelId, UserId};

mod _checks;
mod _onboarding;
mod _utils;
mod admin;
mod botowners;
mod explain;
mod help;
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

struct CollectedGuild {
    guild_id: NonZeroU64,
    guild_name: String,
    owner_id: NonZeroU64,
}

enum StaffPosition {
    Staff,
    Manager,
    HeadManager,
    Developer,
    HeadDeveloper,
}

struct StaffResync {
    user_id: NonZeroU64,
    col: StaffPosition,
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

async fn event_listener(event: &FullEvent, user_data: &Data) -> Result<(), Error> {
    let main_server = std::env::var("MAIN_SERVER")
        .unwrap()
        .parse::<NonZeroU64>()
        .unwrap();
    let testing_server = std::env::var("TESTING_SERVER")
        .unwrap()
        .parse::<NonZeroU64>()
        .unwrap();

    match event {
        FullEvent::InteractionCreate { interaction, ctx: _ } => {
            info!("Interaction received: {:?}", interaction.id());
        }
        FullEvent::Ready {
            data_about_bot,
            ctx,
        } => {
            // Always wait a bit here for cache to finish up
            tokio::time::sleep(Duration::from_secs(2)).await;

            info!(
                "{} is ready! Doing some minor DB fixes",
                data_about_bot.user.name
            );
            sqlx::query!(
                "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE LOWER(claimed_by) = 'none'",
            )
            .execute(&user_data.pool)
            .await?;

            let ctx = ctx.to_owned();
            let pool = user_data.pool.clone();

            let mut interval = tokio::time::interval(Duration::from_millis(30000));

            let lounge_channel_id = ChannelId(
                std::env::var("LOUNGE_CHANNEL")
                    .unwrap()
                    .parse::<NonZeroU64>()
                    .unwrap(),
            );

            let main_server = std::env::var("MAIN_SERVER")
                .unwrap()
                .parse::<NonZeroU64>()
                .unwrap();
            let staff_server = std::env::var("STAFF_SERVER")
                .unwrap()
                .parse::<NonZeroU64>()
                .unwrap();
            let testing_server = std::env::var("TESTING_SERVER")
                .unwrap()
                .parse::<NonZeroU64>()
                .unwrap();

            loop {
                info!("Performing staff recalc");

                let dev_role =
                    poise::serenity_prelude::RoleId(std::env::var("DEV_ROLE")?.parse::<NonZeroU64>()?);
                let head_dev_role =
                    poise::serenity_prelude::RoleId(std::env::var("HEAD_DEV_ROLE")?.parse::<NonZeroU64>()?);
                let staff_man_role = poise::serenity_prelude::RoleId(
                    std::env::var("STAFF_MAN_ROLE")?.parse::<NonZeroU64>()?,
                );
                let head_man_role =
                    poise::serenity_prelude::RoleId(std::env::var("HEAD_MAN_ROLE")?.parse::<NonZeroU64>()?);
                let web_mod_role =
                    poise::serenity_prelude::RoleId(std::env::var("WEB_MOD_ROLE")?.parse::<NonZeroU64>()?);

                let mut staff_resync = Vec::new();

                // Get all members on staff server
                for (_, member) in ctx.cache.guild(staff_server).unwrap().members.iter() {
                    if member.roles.contains(&dev_role) {
                        staff_resync.push(StaffResync {
                            user_id: member.user.id.0,
                            col: StaffPosition::Developer
                        });
                    }
                    if member.roles.contains(&head_dev_role) {
                        staff_resync.push(StaffResync {
                            user_id: member.user.id.0,
                            col: StaffPosition::HeadDeveloper
                        });
                    }
                    if member.roles.contains(&staff_man_role) {
                        staff_resync.push(StaffResync {
                            user_id: member.user.id.0,
                            col: StaffPosition::Manager
                        });
                    }
                    if member.roles.contains(&head_man_role) {
                        staff_resync.push(StaffResync {
                            user_id: member.user.id.0,
                            col: StaffPosition::HeadManager
                        });
                    }
                    if member.roles.contains(&web_mod_role) {
                        staff_resync.push(StaffResync {
                            user_id: member.user.id.0,
                            col: StaffPosition::Staff
                        });
                    }
                }

                // Create a transaction
                let mut tx = pool.begin().await?;

                // First unset all staff
                sqlx::query!("UPDATE users SET staff = false, ibldev = false, iblhdev = false, admin = false, hadmin = false")
                .execute(&mut tx)
                .await?;

                // Now set all staff as per the staff_resync vector
                for staff in staff_resync {
                    match staff.col {
                        StaffPosition::Staff => {
                            sqlx::query!("UPDATE users SET staff = true WHERE user_id = $1", staff.user_id.to_string())
                                .execute(&mut tx)
                                .await?;
                        },
                        StaffPosition::Manager => {
                            sqlx::query!("UPDATE users SET staff = true, admin = true WHERE user_id = $1", staff.user_id.to_string())
                                .execute(&mut tx)
                                .await?;
                        },
                        StaffPosition::Developer => {
                            sqlx::query!("UPDATE users SET staff = true, ibldev = true WHERE user_id = $1", staff.user_id.to_string())
                                .execute(&mut tx)
                                .await?;
                        },
                        StaffPosition::HeadDeveloper => {
                            sqlx::query!("UPDATE users SET staff = true, ibldev = true, iblhdev = true WHERE user_id = $1", staff.user_id.to_string())
                                .execute(&mut tx)
                                .await?;
                        },
                        StaffPosition::HeadManager => {
                            sqlx::query!("UPDATE users SET staff = true, admin = true, hadmin = true WHERE user_id = $1", staff.user_id.to_string())
                                .execute(&mut tx)
                                .await?;
                        }
                    } 
                }

                // Commit the transaction
                tx.commit().await?;

                interval.tick().await;

                info!("Checking for claimed bots greater than 1 hour claim interval");

                let res = sqlx::query!(
                    "SELECT bot_id, claimed_by, last_claimed, owner FROM bots WHERE type = 'claimed' AND NOW() - last_claimed > INTERVAL '1 hour'",
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
                            "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
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
                            "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
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
                        "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
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
                    let msg = CreateMessage::default()
                        .content(format!("<@{}>", claimed_by))
                        .embed(
                            CreateEmbed::default()
                                .title("Auto-Unclaimed Bot")
                                .description(
                                    format!(
                                        "Bot <@{}> was auto-unclaimed (was previously claimed by <@{}> due to it being claimed for over one hour without being approved or denied).\nThis bot was last claimed at {} ({}).", 
                                        bot.bot_id,
                                        claimed_by,
                                        last_claimed.format("%Y-%m-%d %H:%M:%S"),
                                        (start_time - last_claimed).num_minutes().to_string() + " minutes ago"
                                    ))
                                .color(0xFF0000)
                        );

                    let err = lounge_channel_id.send_message(&ctx, msg).await;

                    if err.is_err() {
                        error!(
                            "Error while sending message to lounge: {:?}",
                            err.unwrap_err()
                        );
                        continue;
                    }

                    let owner = bot.owner.parse::<NonZeroU64>();

                    if let Ok(owner) = owner {
                        let private_channel = UserId(owner).create_dm_channel(&ctx).await;

                        if private_channel.is_err() {
                            error!(
                                "Error while sending message to owner: {:?}",
                                private_channel.unwrap_err()
                            );
                            continue;
                        }

                        let private_channel = private_channel.unwrap();

                        let msg = CreateMessage::default()
                            .embed(
                                CreateEmbed::default()
                                    .title("Bot Unclaimed!")
                                    .description(
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
                                        ))
                                    .footer(CreateEmbedFooter::new("This is completely normal, don't worry!"))
                            );

                        let err = private_channel.send_message(&ctx, msg).await;

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

                // Loop through all guilds
                let guilds = ctx.cache.guilds();

                let http = ctx.http.clone();

                let mut collected_guilds = Vec::new();

                // We do this to avoid the async cache guard introduced in serenity next
                for guild_id in guilds {
                    // Check if guild is official (main/testing/staff)
                    if guild_id.0 == main_server
                        || guild_id.0 == testing_server
                        || guild_id.0 == staff_server
                    {
                        continue;
                    }

                    let guild = guild_id.to_guild_cached(&ctx);

                    if guild.is_none() {
                        continue;
                    }

                    let guild = guild.unwrap();

                    // Collect the guild
                    collected_guilds.push(CollectedGuild {
                        guild_id: guild_id.0,
                        owner_id: guild.owner_id.0,
                        guild_name: guild.name.clone(),
                    });
                }

                // Get the current bot ID (for checking ownership here)
                let bowner = ctx.cache.current_user().id.0;

                for collected in collected_guilds {
                    // Try parsing guild name to u64
                    let guild_u64 = collected.guild_name.parse::<NonZeroU64>();

                    if guild_u64.is_err() {
                        // We have a bad guild name, delete or leave if we are not owner
                        info!(
                            "Deleting guild {} because it is not a valid guild name: {}",
                            collected.guild_id, collected.guild_name
                        );

                        if bowner == collected.owner_id {
                            let err = http.delete_guild(GuildId(collected.guild_id)).await;

                            if err.is_err() {
                                error!(
                                    "Error while deleting guild {}: {:?}",
                                    collected.guild_id,
                                    err.unwrap_err()
                                );
                                continue;
                            }
                        } else {
                            let err = http.leave_guild(GuildId(collected.guild_id)).await;

                            if err.is_err() {
                                error!(
                                    "Error while leaving guild {}: {:?}",
                                    collected.guild_id,
                                    err.unwrap_err()
                                );
                                continue;
                            }
                        }
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
                                    if collected.owner_id == bowner {
                                        let err =
                                            http.delete_guild(GuildId(collected.guild_id)).await;

                                        if err.is_err() {
                                            error!(
                                                "Error while deleting guild with ID: {:?} (error: {:?})",
                                                collected.guild_id,
                                                err.unwrap_err()
                                            );
                                        }
                                    } else {
                                        let err =
                                            http.leave_guild(GuildId(collected.guild_id)).await;

                                        if err.is_err() {
                                            error!(
                                                "Error while leaving guild with ID: {:?} (error: {:?})",
                                                collected.guild_id,
                                                err.unwrap_err()
                                            );
                                        }
                                    }
                                    continue;
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
        FullEvent::CacheReady { guilds, ctx: _ } => {
            info!("Cache ready with {} guilds", guilds.len());
        }
        FullEvent::GuildMemberAddition { new_member, ctx } => {
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

#[tokio::main]
async fn main() {
    const MAX_CONNECTIONS: u32 = 3; // max connections to the database, we don't need too many here

    dotenv().ok();

    env_logger::init();

    // proxy url is always http://localhost:3219
    let mut proxy_url = "http://127.0.0.1:3219/".to_string();
    if let Ok(v) = std::env::var("PROXY_URL") {
        proxy_url = v;
    }

    info!("Proxy URL: {}", proxy_url);

    let http =
        serenity::HttpBuilder::new(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
            .proxy(proxy_url)
            .ratelimiter_disabled(true)
            .build();

    let client_builder =
        serenity::ClientBuilder::new_with_http(http, serenity::GatewayIntents::all());

    let framework = poise::Framework::new(
        poise::FrameworkOptions {
            initialize_owners: true,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("ibb!".into()),
                ..poise::PrefixFrameworkOptions::default()
            },
            listener: |event, _ctx, user_data| Box::pin(event_listener(event, user_data)),
            commands: vec![
                age(),
                actf(),
                register(),
                simplehelp(),
                help::help(),
                explain::explainme(),
                staff::staff(),
                testing::onboard(),
                testing::invite(),
                testing::claim(),
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
                admin::votereset(),
                admin::voteresetall(),
                admin::onboardman(),
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
    );

    let mut client = client_builder
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

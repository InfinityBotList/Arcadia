use crate::{_checks as checks, _onboarding::onboard_autocomplete};
use crate::_utils::Bool;
use futures_util::StreamExt;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::UserId;
use serde::Serialize;
use log::{error, info};
use std::time::Duration;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

#[derive(Serialize)]
struct Reason {
    reason: String,
}

/// Gets the invite to a bot
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing")]
pub async fn invite(
    ctx: Context<'_>,
    #[description = "The invite to the bot"] bot: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let invite_data = sqlx::query!(
        "SELECT invite FROM bots WHERE bot_id = $1 OR queue_name ILIKE $1 OR vanity = $1 ORDER BY created_at DESC LIMIT 1",
        bot
    )
    .fetch_one(&data.pool)
    .await?;

    ctx.say(&format!("Invite: {}", invite_data.invite)).await?;
    Ok(())
}

/// Starts the onboarding process in the newly created server
#[poise::command(prefix_command, user_cooldown = 10, category = "Testing")]
pub async fn onboard(ctx: Context<'_>) -> Result<(), Error> {
    if !crate::_onboarding::handle_onboarding(ctx, false, None).await? {
        return Ok(());
    }

    Ok(())
}

/// Sends the staff guide link
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 10,
    category = "Testing"
)]
pub async fn staffguide(ctx: Context<'_>) -> Result<(), Error> {
    if !crate::_onboarding::handle_onboarding(ctx, false, None).await? {
        return Ok(());
    }

    ctx.say("The staff guide can be found at https://ptb.botlist.app/staff/guide. Please **do not** bookmark this page as the URL may change in the future").await?;

    Ok(())
}

/// Checks the bot queue
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing")]
pub async fn queue(
    ctx: Context<'_>,
    #[description = "Whether to embed or not"] embed: Option<Bool>,
) -> Result<(), Error> {
    let embed = embed.unwrap_or(Bool::True).to_bool();

    if !crate::_onboarding::handle_onboarding(ctx, embed, None).await? {
        return Ok(());
    }

    let data = ctx.data();

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(&data.pool)
    .await?;

    let bots = sqlx::query!(
        "SELECT claimed_by, bot_id, approval_note, short, queue_name, owner FROM bots WHERE type = 'pending' OR type = 'claimed' ORDER BY created_at ASC",
    )
    .fetch_all(&data.pool)
    .await?;

    if bots.is_empty() {
        ctx.say("There are no bots in the queue!").await?;
        return Ok(());
    }

    let mut current_bot = 0;
    let bot_len = bots.len();


    // Send message with buttons
    let mut msg = ctx.send(|m| {
        let bot = &bots[current_bot];

        let text_msg = format!("**{name} [{c_bot}/{bot_len}]**\n**ID:** {id}\n**Claimed by:** {claimed_by}\n**Approval note:** {approve_note}\n**Short:** {short}\n**Queue name:** {name}\n**Owner:** {owner}", 
            name = bot.queue_name,
            c_bot = current_bot + 1, 
            bot_len = bot_len,
            id = bot.bot_id, 
            claimed_by = bot.claimed_by.clone().unwrap_or_else(|| "*You are free to test this bot. It is not claimed*".to_string()), 
            approve_note = bot.approval_note, 
            short = bot.short,
            owner = bot.owner
        );

        if !embed {
            m.content(text_msg);
        } else {
            m.embed(
                |e| {
                    e
                    .title(format!("{name} {c_bot}/{bot_len}", name = bot.queue_name, c_bot = current_bot + 1, bot_len = bot_len))
                    .field("ID", bot.bot_id.clone(), false)
                    .field("Short", bot.short.clone(), false)
                    .field("Claimed by", bot.claimed_by.clone().unwrap_or_else(|| "*You are free to test this bot. It is not claimed*".to_string()), false)
                    .field("Approval note", bot.approval_note.clone(), true)
                    .field("Queue name", bot.queue_name.clone(), true)
                }
            );
    }

        m.components(|c| {
            c.create_action_row(|ar| {                
                ar.create_button(|b| {
                    b.label("Prev")
                    .style(serenity::ButtonStyle::Primary)
                    .custom_id("q:prev")
                    .disabled(current_bot <= 0)
                });

                ar.create_button(|b| {
                    b.label("Cancel")
                    .style(serenity::ButtonStyle::Danger)
                    .custom_id("q:cancel")
                });

                ar.create_button(|b| {
                    b.label("Next")
                    .style(serenity::ButtonStyle::Primary)
                    .custom_id("q:next")
                    .disabled(current_bot >= bot_len - 1)
                });

                ar
            })
        })
    })
    .await?
    .into_message()
    .await?;

    let mut interaction = msg
    .await_component_interactions(ctx.serenity_context())
    .author_id(ctx.author().id)
    .timeout(Duration::from_secs(120))
    .build();

    while let Some(item) = interaction.next().await {
        item.defer(&ctx.serenity_context()).await?;

        let id = &item.data.custom_id;

        info!("Received interaction: {}", id);

        if id == "q:cancel" {
            item.delete_original_interaction_response(ctx.serenity_context())
                .await?;
            interaction.stop();
            break;
        }

        if id == "q:prev" {
            if current_bot == 0 {
                current_bot = 0
            }

            current_bot -= 1;
        } else if id == "q:next" {
            if current_bot >= bot_len - 1 {
                current_bot = bot_len - 1
            }

            current_bot += 1
        }

        msg.edit(ctx, |m| {
            let bot = &bots[current_bot];
    
            let text_msg = format!("**{name} [{c_bot}/{bot_len}]**\n**ID:** {id}\n**Claimed by:** {claimed_by}\n**Approval note:** {approve_note}\n**Short:** {short}\n**Queue name:** {name}\n**Owner:** {owner}", 
                name = bot.queue_name,
                c_bot = current_bot + 1, 
                bot_len = bot_len,
                id = bot.bot_id, 
                claimed_by = bot.claimed_by.clone().unwrap_or_else(|| "*You are free to test this bot. It is not claimed*".to_string()), 
                approve_note = bot.approval_note, 
                short = bot.short,
                owner = bot.owner
            );
            
            if !embed {
                m.content(text_msg);
            } else {
                m.embed(
                    |e| {
                        e
                        .title(format!("{name} {c_bot}/{bot_len}", name = bot.queue_name, c_bot = current_bot + 1, bot_len = bot_len))
                        .field("ID", bot.bot_id.clone(), false)
                        .field("Short", bot.short.clone(), false)
                        .field("Claimed by", bot.claimed_by.clone().unwrap_or_else(|| "*You are free to test this bot. It is not claimed*".to_string()), false)
                        .field("Approval note", bot.approval_note.clone(), true)
                        .field("Queue name", bot.queue_name.clone(), true)
                    }
                );
                }
    
            m.components(|c| {
                c.create_action_row(|ar| {                
                    ar.create_button(|b| {
                        b.label("Prev")
                        .style(serenity::ButtonStyle::Primary)
                        .custom_id("q:prev")
                        .disabled(current_bot <= 0)
                    });
    
                    ar.create_button(|b| {
                        b.label("Cancel")
                        .style(serenity::ButtonStyle::Danger)
                        .custom_id("q:cancel")
                    });
    
                    ar.create_button(|b| {
                        b.label("Next")
                        .style(serenity::ButtonStyle::Primary)
                        .custom_id("q:next")
                        .disabled(current_bot >= bot_len - 1)
                    });
    
                    ar
                })
            })    
        }).await?;
    }

    Ok(())
}

/// Implementation of the claim command
pub async fn claim_impl(ctx: Context<'_>, bot: &libavacado::types::DiscordUser) -> Result<(), Error> {    
    if !crate::_onboarding::handle_onboarding(ctx, false, Some(&bot.id.to_string())).await? {
        return Ok(());
    }

    let test_bot_id = std::env::var("TEST_BOT")?;

    if !checks::is_staff(ctx).await? {
        return Err("You must be staff to use this command!".into());
    }

    if bot.id == test_bot_id {
        return Err("You cannot claim the test bot!".into());
    }

    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    // Check if its claimed by someone
    let data = ctx.data();
    let discord = ctx.serenity_context();

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(&data.pool)
    .await?;

    let claimed = sqlx::query!(
        "SELECT type, owner, claimed_by FROM bots WHERE bot_id = $1",
        bot.id
    )
    .fetch_one(&data.pool)
    .await?;

    if claimed.r#type != "pending" {
        return Err("This bot is not pending review".into());
    }

    // Get main owner
    let owner = UserId(claimed.owner.parse::<u64>()?);

    if claimed.claimed_by.is_none() || claimed.claimed_by.as_ref().unwrap().is_empty() {
        // Claim it
        sqlx::query!(
            "UPDATE bots SET type = 'claimed', last_claimed = NOW(), claimed_by = $1 WHERE bot_id = $2",
            ctx.author().id.0.to_string(),
            bot.id
        )
        .execute(&data.pool)
        .await?;

        libavacado::staff::add_action_log(
            &data.pool,
            &bot.id,
            &ctx.author().id.0.to_string(),
            "Claimed",
            "claim",
        )
        .await?;

        ctx.send(|m| {
            m.embed(|e| {
                e.title("Bot Claimed")
                    .description(format!("You have claimed {}", bot.username))
                    .footer(|f| f.text("Use ibb!invite or /invite to get the bots invite"))
            })
        })
        .await?;

        let private_channel = owner.create_dm_channel(discord).await?;

        private_channel
            .send_message(discord, |m| {
                m.embed(|e| {
                    e.title("Bot Claimed!");
                    e.description(format!(
                        "<@{}> has claimed <@{}>",
                        ctx.author().id.0,
                        bot.id
                    ));
                    e.footer(|f| {
                        f.text("This is completely normal, don't worry!");
                        f
                    });
                    e
                });
                m
            })
            .await?;
    } else {
        let mut msg = ctx
            .send(|m| {
                m.embed(|e| {
                    e.title("Bot Already Claimed");
                    e.description(format!(
                        "This bot is already claimed by <@{}>",
                        claimed.claimed_by.as_ref().unwrap()
                    ));
                    e.color(0xFF0000);
                    e
                });

                m.components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.custom_id("fclaim")
                                .style(serenity::ButtonStyle::Primary)
                                .label("Force Claim")
                        });
                        r.create_button(|b| {
                            b.custom_id("remind")
                                .style(serenity::ButtonStyle::Secondary)
                                .label("Remind Reviewer")
                        })
                    });

                    c
                });

                m
            })
            .await?
            .into_message()
            .await?;

        let interaction = msg
            .await_component_interaction(ctx.serenity_context())
            .author_id(ctx.author().id)
            .await;
        msg.edit(ctx.serenity_context(), |b| b.components(|b| b)).await?; // remove buttons after button press

        if let Some(m) = &interaction {
            let id = &m.data.custom_id;

            let claimed_by = claimed.claimed_by.unwrap();

            if id == "remind" {
                libavacado::staff::add_action_log(
                    &data.pool,
                    &bot.id,
                    &claimed_by,
                    "User reminder",
                    "reminder",
                )
                .await?;
                ctx.say(
                    format!(
                        "<@{claimed_by}>, did you forgot to finish testing <@{bot_id}>? This reminder has been recorded internally for staff activity tracking purposes!", 
                        claimed_by = claimed_by,
                        bot_id = bot.id
                    )
                ).await?;
            } else {
                // Force claim
                sqlx::query!(
                    "UPDATE bots SET type = 'claimed', last_claimed = NOW(), claimed_by = $1 WHERE bot_id = $2",
                    ctx.author().id.0.to_string(),
                    bot.id
                )
                .execute(&data.pool)
                .await?;

                libavacado::staff::add_action_log(
                    &data.pool,
                    &bot.id,
                    &ctx.author().id.0.to_string(),
                    "Force claim since previous staff did not finish reviewing bot",
                    "claim",
                )
                .await?;

                let private_channel = owner.create_dm_channel(discord).await?;

                private_channel
                    .send_message(discord, |m| {
                        m.embed(|e| {
                            e.title("Bot Reclaimed!");
                            e.description(format!(
                                "<@{}> has reclaimed <@{}> from <{}>",
                                ctx.author().id.0,
                                bot.id,
                                claimed_by
                            ));
                            e.footer(|f| {
                                f.text("This is completely normal, don't worry!");
                                f
                            });
                            e
                        });
                        m
                    })
                    .await?;

                ctx.say(format!(
                    "You have claimed <@{bot_id}> and the bot owner has been notified!",
                    bot_id = bot.id
                ))
                .await?;
            }
        } else {
            return Ok(());
        }

        return Ok(());
    }

    Ok(())
}

/// Claims a bot
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 3,
    category = "Testing",
    check = "checks::is_staff"
)]
pub async fn claim(
    ctx: Context<'_>,
    #[autocomplete = "claim_autocomplete"]
    #[description = "The bot you wish to claim"] bot: String,
) -> Result<(), Error> {
    let mut resolved_id = bot;

    if resolved_id.starts_with("<@") {
        resolved_id = resolved_id.replace("<@", "");
        resolved_id = resolved_id.replace(">", "");
    }

    // Try parsing as a user
    let user = resolved_id.parse::<u64>();

    if user.is_err() {
        return Err("Invalid user ID".into());
    }

    let public = ctx.data();

    let user = libavacado::public::get_user(&public.avacado_public, &resolved_id, false).await?;

    claim_impl(ctx, user.as_ref()).await?;

    Ok(())
}

async fn claim_autocomplete<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<poise::AutocompleteChoice<String>> {
    info!("Called claim autocomplete");

    let onboard_ac = onboard_autocomplete(ctx, partial).await;

    if let Ok(v) = onboard_ac {
        if !v.is_empty() {
            return v;
        }
    } else {
        let err = onboard_ac.err().unwrap();
        error!("Error getting onboard autocomplete: {:?}", err);
        return Vec::new();
    }

    let data = ctx.data();

    let bots = sqlx::query!(
        "SELECT bot_id, queue_name FROM bots WHERE (bot_id ILIKE $1 OR vanity ILIKE $1) AND type = 'pending'",
        format!("%{}%", partial)
    )
    .fetch_all(&data.pool)
    .await;

    if bots.is_err() {
        error!("Error getting bots: {:?}", bots);
        return vec![];
    }

    let bots = bots.unwrap();

    let mut out = vec![];

    let test_bot_id = std::env::var("TEST_BOT").unwrap();
    for bot in bots {
        if bot.bot_id == test_bot_id {
            continue
        }

        out.push(poise::AutocompleteChoice {
            name: format!("{} ({})", bot.queue_name, bot.bot_id),
            value: bot.bot_id,
        });
    }

    out
}

#[poise::command(
    context_menu_command = "Claim Bot",
    user_cooldown = 3,
    category = "Testing",
    check = "checks::is_staff"
)]
pub async fn claim_context(
    ctx: Context<'_>,
    #[description = "User"] user: serenity::User,
) -> Result<(), Error> {
    claim_impl(ctx, &libavacado::types::DiscordUser::from_user(user)).await
}

pub async fn unclaim_impl(ctx: Context<'_>, bot: serenity::User) -> Result<(), Error> {
    let data = ctx.data();
    let discord = ctx.serenity_context();

    if !crate::_onboarding::handle_onboarding(ctx, false, None).await? {
        return Ok(());
    }

    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(&data.pool)
    .await?;

    let claimed = sqlx::query!(
        "SELECT claimed_by, owner FROM bots WHERE bot_id = $1",
        bot.id.0.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    // Get main owner
    let owner = UserId(claimed.owner.parse::<u64>()?);

    if claimed.claimed_by.is_none() || claimed.claimed_by.as_ref().unwrap().is_empty() {
        ctx.say(format!("<@{}> is not claimed", bot.id.0)).await?;
    } else {
        sqlx::query!(
            "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
            bot.id.0.to_string()
        )
        .execute(&data.pool)
        .await?;

        libavacado::staff::add_action_log(
            &data.pool,
            &bot.id.0.to_string(),
            &ctx.author().id.0.to_string(),
            "Unclaimed bot",
            "unclaim",
        )
        .await?;

        let private_channel = owner.create_dm_channel(discord).await?;

        private_channel
            .send_message(discord, |m| {
                m.embed(|e| {
                    e.title("Bot Unclaimed!")
                        .description(format!(
                            "<@{}> has unclaimed <@{}>",
                            ctx.author().id.0,
                            bot.id.0
                        ))
                        .footer(|f| f.text("This is completely normal, don't worry!"))
                })
            })
            .await?;

        ctx.say(format!("You have unclaimed <@{}>", bot.id.0))
            .await?;
    }

    Ok(())
}

/// Unclaims a bot
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 3,
    category = "Testing",
    check = "checks::is_staff"
)]
pub async fn unclaim(
    ctx: Context<'_>,
    #[description = "The bot you wish to unclaim"] bot: serenity::Member,
) -> Result<(), Error> {
    unclaim_impl(ctx, bot.user).await
}

#[poise::command(
    context_menu_command = "Unclaim Bot",
    user_cooldown = 3,
    category = "Testing",
    check = "checks::is_staff"
)]
pub async fn unclaim_context(
    ctx: Context<'_>,
    #[description = "User"] user: serenity::User,
) -> Result<(), Error> {
    unclaim_impl(ctx, user).await
}

/// Approves a bot
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 3,
    category = "Testing",
    check = "checks::is_staff"
)]
pub async fn approve(
    ctx: Context<'_>,
    #[description = "The bot you wish to approve"] bot: serenity::Member,
    #[description = "The reason for approval"] reason: String,
) -> Result<(), Error> {
    if !crate::_onboarding::handle_onboarding(ctx, false, Some(&reason)).await? {
        return Ok(());
    }
    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    libavacado::staff::approve_bot(
        &ctx.serenity_context(),
        &ctx.data().pool,
        &bot.user.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("Approved bot").await?;

    Ok(())
}

/// Denies a bot
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 10,
    category = "Testing",
    check = "checks::is_staff"
)]
pub async fn deny(
    ctx: Context<'_>,
    #[description = "The bot you wish to deny"] bot: serenity::User,
    #[description = "The reason for denial"] reason: String,
) -> Result<(), Error> {
    if !crate::_onboarding::handle_onboarding(ctx, false, Some(&reason)).await? {
        return Ok(());
    }

    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    libavacado::staff::deny_bot(
        &ctx.serenity_context(),
        &ctx.data().pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("Denied bot").await?;

    Ok(())
}

use crate::_checks as checks;
use crate::_utils::Bool;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::UserId;
use serde::Serialize;
use std::fmt::Write;

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
        "SELECT invite FROM bots WHERE bot_id = $1 OR name = $1 OR vanity = $1",
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

    ctx.say("The staff guide can be found at https://seed.infinitybots.gg/sovngarde. Please **do not** bookmark this page as the URL may change in the future").await?;

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
        "UPDATE bots SET claimed_by = NULL, claimed = false WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(&data.pool)
    .await?;

    let bots = sqlx::query!(
        "SELECT claimed_by, bot_id, approval_note, name FROM bots WHERE type = 'pending'",
    )
    .fetch_all(&data.pool)
    .await?;

    if bots.is_empty() {
        return Err("There are no bots in the queue!".into());
    }

    let i = 1;

    let mut desc_str = "".to_string();

    let page = 1;

    for bot in bots {
        if let Some(claimed_by) = bot.claimed_by {
            writeln!(
                desc_str,
                "**{i}.** {name} ({bot_id}) [Claimed by: {claimed_by} (<@{claimed_by}>)]\n**Note:** {ap_note}",
                i = i,
                name = bot.name,
                bot_id = bot.bot_id,
                claimed_by = claimed_by,
                ap_note = bot.approval_note.unwrap_or_else(|| "None".to_string()),
            )?;
        } else {
            writeln!(
                desc_str,
                "**{i}.** {name} ({bot_id}) [Unclaimed]\n**Note**: {ap_note}",
                i = i,
                name = bot.name,
                bot_id = bot.bot_id,
                ap_note = bot.approval_note.unwrap_or_else(|| "None".to_string()),
            )?;
        }

        if desc_str.len() > 1998 {
            if embed {
                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Bot Queue (Page".to_string() + &page.to_string() + ")")
                            .description(&desc_str)
                            .footer(|f| f.text("Use ibb!invite or /invite to get the bots invite"))
                            .color(0xA020F0)
                    })
                })
                .await?;
            } else {
                ctx.say(desc_str.clone() + "\n\nUse ibb!invite or /invite to get the bots invite")
                    .await?;
            }

            desc_str = "".to_string();
        }
    }

    if !desc_str.is_empty() {
        if embed {
            ctx.send(|m| {
                m.embed(|e| {
                    e.title("Bot Queue (Page".to_string() + &page.to_string() + ")")
                        .description(desc_str)
                        .footer(|f| f.text("Use ibb!invite or /invite to get the bots invite"))
                        .color(0xA020F0)
                })
            })
            .await?;
        } else {
            ctx.say(desc_str + "\n\nUse ibb!invite or /invite to get the bots invite")
                .await?;
        }
    }

    Ok(())
}

/// Implementation of the claim command
pub async fn claim_impl(ctx: Context<'_>, bot: serenity::User) -> Result<(), Error> {
    if !crate::_onboarding::handle_onboarding(ctx, false, Some(&bot.id.to_string())).await? {
        return Ok(());
    }

    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    // Check if its claimed by someone
    let data = ctx.data();
    let discord = ctx.discord();

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, claimed = false WHERE LOWER(claimed_by) = 'none'",
    )
    .execute(&data.pool)
    .await?;

    let claimed = sqlx::query!(
        "SELECT type, owner, claimed_by FROM bots WHERE bot_id = $1",
        bot.id.0.to_string()
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
            "UPDATE bots SET claimed = true, last_claimed = NOW(), claimed_by = $1 WHERE bot_id = $2",
            ctx.author().id.0.to_string(),
            bot.id.0.to_string()
        )
        .execute(&data.pool)
        .await?;

        libavacado::staff::add_action_log(
            &data.pool,
            &bot.id.0.to_string(),
            &ctx.author().id.0.to_string(),
            "Claimed",
            "claim",
        )
        .await?;

        ctx.send(|m| {
            m.embed(|e| {
                e.title("Bot Claimed")
                    .description(format!("You have claimed {}", bot.name))
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
                        bot.id.0
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
            .await_component_interaction(ctx.discord())
            .author_id(ctx.author().id)
            .await;
        msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press

        if let Some(m) = &interaction {
            let id = &m.data.custom_id;

            let claimed_by = claimed.claimed_by.unwrap();

            if id == "remind" {
                libavacado::staff::add_action_log(
                    &data.pool,
                    &bot.id.0.to_string(),
                    &claimed_by,
                    "User reminder",
                    "reminder",
                )
                .await?;
                ctx.say(
                    format!(
                        "<@{claimed_by}>, did you forgot to finish testing <@{bot_id}>? This reminder has been recorded internally for staff activity tracking purposes!", 
                        claimed_by = claimed_by,
                        bot_id = bot.id.0
                    )
                ).await?;
            } else {
                // Force claim
                sqlx::query!(
                    "UPDATE bots SET claimed = true, last_claimed = NOW(), claimed_by = $1 WHERE bot_id = $2",
                    ctx.author().id.0.to_string(),
                    bot.id.0.to_string()
                )
                .execute(&data.pool)
                .await?;

                libavacado::staff::add_action_log(
                    &data.pool,
                    &bot.id.0.to_string(),
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
                                bot.id.0,
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
                    bot_id = bot.id.0
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
    #[description = "The bot you wish to claim"] bot: serenity::Member,
) -> Result<(), Error> {
    claim_impl(ctx, bot.user).await
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
    claim_impl(ctx, user).await
}

pub async fn unclaim_impl(ctx: Context<'_>, bot: serenity::User) -> Result<(), Error> {
    let data = ctx.data();
    let discord = ctx.discord();

    if !crate::_onboarding::handle_onboarding(ctx, false, None).await? {
        return Ok(());
    }

    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    sqlx::query!(
        "UPDATE bots SET claimed_by = NULL, claimed = false WHERE LOWER(claimed_by) = 'none'",
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
            "UPDATE bots SET claimed_by = NULL, claimed = false WHERE bot_id = $1",
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
        &ctx.discord(),
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
        &ctx.discord(),
        &ctx.data().pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("Denied bot").await?;

    Ok(())
}

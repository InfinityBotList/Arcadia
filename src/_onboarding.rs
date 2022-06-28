use log::info;
use poise::serenity_prelude::Mentionable;

use poise::serenity_prelude as serenity;
use serde::Serialize;

#[derive(Serialize)]
struct SectionQuestion {
    /// Name of section
    name: String,
    /// This is inputted by users
    answer: String, 
    subsections: Vec<SectionQuestion>,
}
#[derive(Serialize)]
struct OnboardingQuiz {
    sections: Vec<SectionQuestion>,
}

impl OnboardingQuiz {
    fn new() -> OnboardingQuiz {
        OnboardingQuiz {
            sections: vec![
                SectionQuestion {
                    name: "about".to_string(),
                    answer: "".to_string(),
                    subsections: vec![
                        SectionQuestion {
                            name: "ping".to_string(),
                            answer: "".to_string(),
                            subsections: vec![],
                        },
                        SectionQuestion {
                            name: "about".to_string(),
                            answer: "".to_string(),
                            subsections: vec![],
                        },
                        SectionQuestion {
                            name: "cmdinfo".to_string(),
                            answer: "".to_string(),
                            subsections: vec![],
                        },
                        SectionQuestion {
                            name: "globallookup".to_string(),
                            answer: "".to_string(),
                            subsections: vec![],
                        },
                        SectionQuestion {
                            name: "randomcat".to_string(),
                            answer: "".to_string(),
                            subsections: vec![],
                        },
                        SectionQuestion {
                            name: "randomdog".to_string(),
                            answer: "".to_string(),
                            subsections: vec![],
                        },
                    ],
                }
            ],
        }
    }
}

/// Tries to check if onboarding is required, returns ``false`` if command should stop
pub async fn handle_onboarding(
    ctx: crate::Context<'_>,
    user_id: &str,
    set_onboard_state: Option<String>,
) -> Result<bool, crate::Error> {
    if !crate::checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let cmd_name = ctx.command().name;

    let onboard_name = ctx.author().name.clone() + "-onboarding";

    info!("{}", cmd_name);

    let data = ctx.data();
    let discord = ctx.discord();

    let onboard_state = if set_onboard_state.is_some() {
        set_onboard_state.unwrap()
    } else {
        let res = sqlx::query!(
            "SELECT staff_onboard_state FROM users WHERE user_id = $1",
            user_id
        )
        .fetch_one(&data.pool)
        .await?;

        res.staff_onboard_state
    };

    let mut onboard_state = onboard_state.as_str();

    let onboarded = sqlx::query!(
        "SELECT staff_onboarded, staff_onboard_last_start_time FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(&data.pool)
    .await?;

    if onboard_state == "complete" {
        return Ok(true);
    }

    let cur_channel = ctx.channel_id().name(discord).await;

    if let Some(cur_channel) = cur_channel {
        if cur_channel != onboard_name && onboard_state != "pending" {
            ctx.say("You are not in the created onboarding channel!").await?;

            let channel = ctx.guild().unwrap().channel_id_from_name(discord, &onboard_name);

            if channel == None {
                ctx.say("Onboarding channel does not exist, creating!").await?;

                ctx.guild_id().unwrap().create_channel(discord, |c| {
                    c.name(&onboard_name)
                }).await?;    

                return Ok(false);
            }

            return Ok(false);
        }
    } else {
        ctx.say("Could not find an current channel!").await?;

        return Ok(false);
    }

    if cmd_name == "staffguide" && onboard_state == "queue-step" {
        // We are now in staff_onboard_state of staff-guide, set that
        sqlx::query!(
            "UPDATE users SET staff_onboard_state = 'staff-guide-viewed' WHERE user_id = $1",
            user_id
        )
        .execute(&data.pool)
        .await?;
        return Ok(true);
    }

    // Allow users to see queue again
    if cmd_name == "queue" && !vec!["pending", "complete"].contains(&onboard_state) {
        // Check that they are in stage 2 of queue command
        if vec!["claimed-bot"].contains(&onboard_state) {
            onboard_state = "claimed-bot";
        } else {
            onboard_state = "queue-step";
        }
    }

    if onboarded.staff_onboarded {
        info!("{} is already onboarded", user_id);
    } else if onboarded.staff_onboard_last_start_time.is_none() {
        sqlx::query!(
            "UPDATE users SET staff_onboard_last_start_time = NOW() WHERE user_id = $1",
            user_id
        )
        .execute(&data.pool)
        .await?;
    } else if chrono::offset::Utc::now() - onboarded.staff_onboard_last_start_time.unwrap()
        > chrono::Duration::hours(1)
    {
        sqlx::query!(
            "UPDATE users SET staff_onboard_last_start_time = NOW(), staff_onboard_state = 'pending' WHERE user_id = $1",
            user_id
        )
        .execute(&data.pool)
        .await?;

        ctx.say(
            "You exceeded the time limit (1 hour) for the previous onboarding attempt. Retrying...",
        )
        .await?;

        onboard_state = "pending";
    }

    let test_bot = std::env::var("TEST_BOT")?;
    let bot_page = std::env::var("BOT_PAGE")?;
    let current_user = ctx.discord().cache.current_user();

    // Before matching, make sure 'Ninja Bot' is always pending
    sqlx::query!(
        "UPDATE bots SET type = 'pending' WHERE bot_id = $1",
        test_bot
    )
    .execute(&data.pool)
    .await?;

    match onboard_state {
        "pending" => {
            ctx.say("**Welcome to Infinity Bot List**\n\nSince you seem new to this place, how about a nice look arou-?").await?;

            ctx.send(|m| {
                m.embed(|e| {
                    e.title("Bot Resubmitted")
                    .description(
                        format!(
                            "**Bot:** {bot_id} ({bot_name})\n\n**Owner:** {owner_id} ({owner_name})\n\n**Bot Page:** {bot_page}",
                            bot_id = "<@".to_string() + &test_bot + ">",
                            bot_name = "Ninja Bot",
                            owner_id = current_user.id.mention(),
                            owner_name = current_user.name,
                            bot_page = bot_page + "/bot/" + &test_bot
                        )
                    )
                    .footer(|f| {
                        f.text("Are you ready to take on your first challenge, young padawan?")
                    })
                    .color(0xA020F0)
                })
            }).await?;

            // Delete a old onboarding channel if it exists
            let channel = ctx.guild().unwrap().channel_id_from_name(discord, &onboard_name);

            if let Some(chan_id) = channel {
                chan_id.delete(discord).await?;
            }

            ctx.guild_id().unwrap().create_channel(discord, |c| {
                c.name(&onboard_name)
            }).await?;

            sqlx::query!(
                "UPDATE users SET staff_onboard_state = 'queue-step' WHERE user_id = $1",
                user_id
            )
            .execute(&data.pool)
            .await?;

            ctx.say("Whoa there! Look at that! There's a new bot to review!!! Type ``/queue`` (or ``ibb!queue``) to see the queue").await?;

            Ok(false)
        }
        "claimed-bot" => {
            if cmd_name == "queue" {
                ctx.say("Not yet implemented").await?;
            } else {
                ctx.say("Type ``/queue`` now to see the queue.").await?;
            }

            Ok(false)
        }
        "queue-step" => {
            if cmd_name == "queue" {
                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Bot Queue (Sandbox Mode)")
                            .description(
                                "**1.** Ninja Bot (".to_string() + &test_bot + ") [Unclaimed]",
                            )
                            .footer(|f| f.text("Use ibb!invite or /invite to get the bots invite"))
                            .color(0xA020F0)
                    })
                })
                .await?;
                ctx.say(r#"
You can use the `/queue` command to see the list of bots pending verification that *you* need to review!

As you can see, ``Ninja Bot`` is whats currently pending review in this training sandbox.

But before we get to reviewing it, lets have a look at the staff guide. You can see the staff guide by using ``/staffguide`` (or ``ibb!staffguide``)"#).await?;
            } else {
                ctx.say("You can use the `/queue` (or ``ibb!queue``) command to see the list of bots pending verification that *you* need to review! Lets try that out?").await?;
            }

            Ok(false)
        }
        // Not for us
        "staff-guide" => Ok(true),
        "staff-guide-viewed" | "staff-guide-viewed-reminded" => {
            if cmd_name == "claim" {
                let mut msg = ctx
                    .send(|m| {
                        m.embed(|e| {
                            e.title("Bot Already Claimed");
                            e.description(format!(
                                "This bot is already claimed by <@{}>",
                                current_user.id
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
                                        .disabled(onboard_state == "staff-guide-viewed")
                                });
                                r.create_button(|b| {
                                    b.custom_id("remind")
                                        .style(serenity::ButtonStyle::Secondary)
                                        .label("Remind Reviewer")
                                        .disabled(onboard_state == "staff-guide-viewed-reminded")
                                })
                            });

                            c
                        });

                        m
                    })
                    .await?
                    .message()
                    .await?;

                if onboard_state == "staff-guide-viewed" {
                    ctx.say("Woah! This bot is already claimed by someone else. Its always best practice to first remind the bot so do that!").await?;
                }

                let interaction = msg
                    .await_component_interaction(ctx.discord())
                    .author_id(ctx.author().id)
                    .await;
                msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press

                if let Some(m) = &interaction {
                    let id = &m.data.custom_id;

                    if id == "remind" {
                        ctx.say(
                            format!(
                                "<@{claimed_by}>, did you forgot to finish testing <@{bot_id}>? This reminder has been recorded internally for staff activity tracking purposes!", 
                                claimed_by = current_user.id,
                                bot_id = test_bot
                            )
                        ).await?;

                        // Create a discord webhook
                        let wh = ctx
                            .channel_id()
                            .create_webhook_with_avatar(
                                discord,
                                "Frostpaw",
                                "https://cdn.infinitybots.xyz/images/png/onboarding-v4.png",
                            )
                            .await?;

                        wh.execute(discord, true, |m| {
                            m.content("Ack! sorry about that. I completely forgot about Ninja Bot due to personal issues")
                        }).await?;

                        ctx.say("Great! With a real bot, things won't go this smoothly, but you can always remind people to test their bot! Now try claiming again, but this time use ``Force Claim``").await?;

                        sqlx::query!(
                            "UPDATE users SET staff_onboard_state = 'staff-guide-viewed-reminded' WHERE user_id = $1",
                            user_id
                        )
                        .execute(&data.pool)
                        .await?;
                    } else {
                        sqlx::query!(
                            "UPDATE users SET staff_onboard_state = 'claimed-bot' WHERE user_id = $1",
                            user_id
                        )
                        .execute(&data.pool)
                        .await?;

                        ctx.say(format!(
                            "You have claimed <@{bot_id}> and the bot owner has been notified!",
                            bot_id = test_bot
                        ))
                        .await?;

                        ctx.say("Now try using ``/queue`` (or ``ibb!queue``) to see what the queue looks like now!").await?;
                    }
                }
            } else {
                ctx.say(
                    "You can use the `/claim` (or ``ibb!claim``) command to claim `Ninja Bot` (`"
                        .to_string()
                        + &test_bot
                        + "`)! Lets try that out?",
                )
                .await?;
            }
            Ok(false)
        }
        _ => {
            ctx.say("Unknown onboarding state:".to_string() + onboard_state)
                .await?;
            Ok(false)
        }
    }
}

pub async fn post_command(ctx: crate::Context<'_>) -> Result<(), crate::Error> {
    if !crate::checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let cmd_name = ctx.command().name;

    info!("{}", cmd_name);

    let data = ctx.data();

    let onboard_state = {
        let res = sqlx::query!(
            "SELECT staff_onboard_state FROM users WHERE user_id = $1",
            ctx.author().id.to_string()
        )
        .fetch_one(&data.pool)
        .await?;

        res.staff_onboard_state
    };

    let onboard_state = onboard_state.as_str();

    match onboard_state {
        "staff-guide-viewed" => {
            ctx.send(|m| {
                m.content("Thats a lot isn't it? I'm glad you're ready to take on your first challenge. To get started, claim ``Ninja Bot``. You can use the queue if you need any help finding it!")
                .ephemeral(true)
            }).await?;
            Ok(())
        }
        _ => Ok(()),
    }
}

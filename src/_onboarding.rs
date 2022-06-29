use std::time::Duration;

use log::info;
use poise::serenity_prelude::{Mentionable, Permissions, RoleId};

use poise::serenity_prelude as serenity;
use serde::Serialize;
use serde_json::json;

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
            sections: vec![SectionQuestion {
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
            }],
        }
    }
}

/// Tries to check if onboarding is required, returns ``false`` if command should stop
pub async fn handle_onboarding(
    ctx: crate::Context<'_>,
    user_id: &str,
    embed: bool,
) -> Result<bool, crate::Error> {
    // Get baisc info from ctx for future use
    let cmd_name = ctx.command().name;

    let onboard_name = ctx.author().id.to_string();

    info!("{}", cmd_name);

    let data = ctx.data();
    let discord = ctx.discord();

    // Verify staff first
    if !crate::_checks::is_any_staff(ctx).await? {
        return Ok(true);
    }

    // Get onboard state (staff_onboard_state may be used later but is right now None and it will in the future be used to allow retaking of onboarding)
    let onboard_state = {
        let res = sqlx::query!(
            "SELECT staff_onboard_state FROM users WHERE user_id = $1",
            user_id
        )
        .fetch_one(&data.pool)
        .await?;

        res.staff_onboard_state
    };

    // Reset old onboards
    sqlx::query!(
        "UPDATE users SET staff_onboard_state = 'pending' WHERE staff_onboard_state = 'complete' AND staff = true AND NOW() - staff_onboard_last_start_time > interval '1 month'"
    )
    .execute(&data.pool)
    .await?;

    // Must be mut so we can change it under some cases
    let mut onboard_state = onboard_state.as_str();

    let onboarded = sqlx::query!(
        "SELECT staff, staff_onboarded, staff_onboard_last_start_time FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(&data.pool)
    .await?;

    // Onboarding is complete, exit early
    if onboard_state == "complete" {
        return Ok(true);
    }

    if onboarded.staff_onboarded {
        info!("{} is already onboarded", user_id);
    } else if onboarded.staff_onboard_last_start_time.is_none() {
        // No onboarding record, so we set it to NOW()
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

    let cur_guild = ctx.guild().unwrap().name;

    if cur_guild.to_lowercase() != onboard_name.to_lowercase() {
        ctx.say("Creating new onboarding server for you!").await?;

        // Reset timer, but here we can't do NOW() exactly as otherwise postgres may fail to see it sooo
        sqlx::query!(
            "UPDATE users SET staff_onboard_last_start_time = NOW() WHERE user_id = $1",
            user_id
        )
        .execute(&data.pool)
        .await?;

        // Check for old onboarding server
        let guilds = discord.cache.guilds();

        let mut found = false;

        for guild in guilds.iter() {
            let name = guild.name(&discord);

            if let Some(name) = name {
                if name.to_lowercase() == onboard_name.to_lowercase() {
                    // Create new channel
                    let channel = guild
                        .create_channel(&discord, |c| {
                            c.name("invite-attempt-".to_string() + &crate::_utils::gen_random(6))
                                .kind(serenity::model::channel::ChannelType::Text)
                        })
                        .await?;

                    // Create new invite
                    let invite = channel
                        .create_invite(&discord, |i| {
                            i.max_age(0).max_uses(1).temporary(false).unique(true)
                        })
                        .await?;

                    // Send invite
                    ctx.say(
                        "Please join the onboarding server here and run ``ibb!queue``: "
                            .to_string()
                            + &invite.url(),
                    )
                    .await?;

                    found = true;
                }
            }
        }

        if !found {
            ctx.say(
                "If the onboarding server still does not exist, please DM a Head Administrator",
            )
            .await?;

            let map = json!({
                "name": onboard_name,
            });

            let new_guild = discord.http.create_guild(&map).await?;

            // Create a channel
            let channel = new_guild
                .create_channel(&discord, |c| {
                    c.name("invite-attempt-".to_string() + &crate::_utils::gen_random(6))
                        .kind(serenity::model::channel::ChannelType::Text)
                })
                .await?;

            // Create a invite
            let invite = channel
                .create_invite(&discord, |i| {
                    i.max_age(0).max_uses(1).temporary(false).unique(true)
                })
                .await?;

            // Send invite
            ctx.say(
                "Please join the newly created onboarding server here and run ``ibb!queue``: "
                    .to_string()
                    + &invite.url(),
            )
            .await?;

            return Ok(false);
        }

        return Ok(false);
    } else {
        // Check if user is admin
        let guild = ctx.guild().unwrap();

        info!("{} {:?}", guild.name, guild.members);

        let mut found = false;

        for member in guild.members.iter() {
            // Resolve the users permissions
            if member.0 .0 == ctx.author().id.0 {
                let permissions = member.1.permissions(&discord)?;
                if permissions.administrator() {
                    found = true;
                }
            }
        }

        if !found {
            // Check for admin role
            let guild = ctx.guild().unwrap();

            let mut found = false;

            let mut role_id: Option<RoleId> = None;

            for role in guild.roles.iter() {
                if role.1.name == "Head Administrator" {
                    found = true;
                    role_id = Some(*role.0);
                }
            }

            if !found {
                // Create role
                let guild = ctx.guild().unwrap();

                let role = guild
                    .create_role(&discord, |r| {
                        r.name("Head Administrator")
                            .colour(0x00ff00)
                            .permissions(Permissions::ADMINISTRATOR)
                            .mentionable(true)
                    })
                    .await?;

                role_id = Some(role.id);
            }

            if role_id == None {
                ctx.say("Failed to fetch admin role").await?;
                return Ok(false);
            }

            // Add admin perms
            ctx.author_member()
                .await
                .unwrap()
                .add_role(&discord, role_id.unwrap())
                .await?;

            ctx.say(
                format!(
                    "You will need to reinvite this bot to the server so scopes can be set properly! Use ``https://discord.com/oauth2/authorize?client_id={}&scope=bot%20applications.commands&permissions=8``. Do this now then rerun ``/queue``!",
                    ctx.discord().cache.current_user().id
                )
            ).await?;

            return Ok(false);
        }
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

    // Reset timer
    sqlx::query!(
        "UPDATE users SET staff_onboard_last_start_time = NOW() WHERE user_id = $1",
        user_id
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

            sqlx::query!(
                "UPDATE users SET staff_onboard_state = 'queue-step' WHERE user_id = $1",
                user_id
            )
            .execute(&data.pool)
            .await?;

            ctx.say("Whoa there! Look at that! There's a new bot to review!!! Type ``/queue`` (or ``ibb!queue``) to see the queue").await?;

            ctx.say("**PRO TIP:** This has a time limit of one hour. Progressing through onboarding or using testing commands properly will reset the timer. You will **not** be informed of when your time limit is close to expiry. Changing the name of the server will cause it to be *deleted*").await?;

            Ok(false)
        }
        "claimed-bot" => {
            if cmd_name == "queue" {
                let desc = format!(
                    "**{i}.** {name} ({bot_id}) [Claimed by: {claimed_by} (<@{claimed_by}>)]\n**Note:** {ap_note}",
                    i = 1,
                    name = "Ninja Bot",
                    bot_id = test_bot,
                    claimed_by = ctx.author().id.0,
                    ap_note = "Please test me :heart:"
                );
                if embed {
                    ctx.send(|m| {
                        m.embed(|e| {
                            e.title("Bot Queue (Sandbox Mode)")
                                .description(desc)
                                .footer(|f| {
                                    f.text("Use ibb!invite or /invite to get the bots invite")
                                })
                                .color(0xA020F0)
                        })
                    })
                    .await?;
                } else {
                    ctx.say(desc.clone() + "\n\nUse ibb!invite or /invite to get the bots invite")
                        .await?;
                }

                ctx.say("Great! As you can see, the bot is claimed by you. Now test the bot as per the staff guide").await?;
            } else if cmd_name == "staffguide" {
                return Ok(true);
            } else {
                ctx.say("Type ``/queue`` now to see the queue.").await?;
            }

            Ok(false)
        }
        "queue-step" => {
            if cmd_name == "queue" {
                let desc = format!(
                    "**{i}.** {name} ({bot_id}) [Unclaimed]\n**Note**: {ap_note}",
                    i = 1,
                    name = "Ninja Bot",
                    bot_id = test_bot,
                    ap_note = "Please test me :heart:"
                );
                if embed {
                    ctx.send(|m| {
                        m.embed(|e| {
                            e.title("Bot Queue (Sandbox Mode)")
                                .description(desc)
                                .footer(|f| {
                                    f.text("Use ibb!invite or /invite to get the bots invite")
                                })
                                .color(0xA020F0)
                        })
                    })
                    .await?;
                } else {
                    ctx.say(desc).await?;
                }
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
        "staff-guide-viewed" => Ok(true),
        "staff-guide-read-encouraged" | "staff-guide-viewed-reminded" => {
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
                                        .disabled(onboard_state == "staff-guide-read-encouraged")
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

                if onboard_state == "staff-guide-read-encouraged" {
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

                        tokio::time::sleep(Duration::from_secs(3)).await;

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

                // Special override to allow revisiting the staffguide command
                if cmd_name == "staffguide" {
                    return Ok(true);
                }
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

    let onboard_name = ctx.author().id.to_string();

    let curr_guild = ctx.guild();

    if let Some(guild) = curr_guild {
        if guild.name != onboard_name {
            return Ok(());
        }
    }

    let onboard_state = onboard_state.as_str();

    match onboard_state {
        "staff-guide-viewed" => {
            ctx.send(|m| {
                m.content("Thats a lot isn't it? I'm glad you're ready to take on your first challenge. To get started, **invite ``Ninja Bot`` using ``ibb!invite [ID]`` where [ID] is the ID from the ``queue`` command**, then claim ``Ninja Bot``!")
            }).await?;

            sqlx::query!(
                "UPDATE users SET staff_onboard_state = 'staff-guide-read-encouraged' WHERE user_id = $1",
                ctx.author().id.to_string()
            )
            .execute(&data.pool)
            .await?;

            Ok(())
        }
        _ => Ok(()),
    }
}

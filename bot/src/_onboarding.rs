use std::time::Duration;

use log::{info, error};
use poise::serenity_prelude::{ChannelId, Mentionable, Permissions, RoleId};

use poise::serenity_prelude as serenity;
use serde_json::json;

/// Internal function to handle the special-cased staff_guide command
/// 
/// This internally creates a onboarding 'fragment' which is used to ensure that a user isn't peeping into someone elses staff verification code
/// 
/// This fragment is then used by sovngarde to fetch the full code and add it to the guide.
async fn _handle_staff_guide(
    ctx: crate::Context<'_>,
    user_id: String,
) -> Result<(), crate::Error> {
    // This is the onboard code user needs to input (random_string@CURRENT_TIME)
    let onboard_code = libavacado::public::gen_random(64) + "@" + &chrono::Utc::now().timestamp().to_string();

    // Get first 20 characters of the onboard code as onboard_fragment
    let onboard_fragment = onboard_code.chars().take(20).collect::<String>();
    
    // Set onboard code for user
    let data = ctx.data();

    sqlx::query!(
        "UPDATE users SET staff_onboard_session_code = $1 WHERE user_id = $2",
        onboard_code,
        user_id
    )
    .execute(&data.pool)
    .await?;

    ctx.say(
        format!(
            r#"The staff guide can be found at https://seed.infinitybots.gg/sovngarde?svu={uid}@{ocf}. Please **do not** bookmark this page as the URL may change in the future
            
Thats a lot isn't it? I'm glad you're ready to take on your first challenge. To get started, **invite ``Ninja Bot`` using ``ibb!invite [ID]`` where [ID] is the ID from the ``queue`` command**, then claim ``Ninja Bot``!

**Note that during onboarding, the *5 digit staff verify code present somewhere in the guide* will be reset every time you run the ``staffguide`` command! Always use the latest command invocation for getting the code**
            "#,
            uid = user_id,
            ocf = onboard_fragment,
    )).await?;

    Ok(())
}

/// Tries to check if onboarding is required, returns ``false`` if command should stop
pub async fn handle_onboarding(
    ctx: crate::Context<'_>,
    embed: bool,
    reason: Option<&str>, // Only applicable for testing-bot
) -> Result<bool, crate::Error> {
    // Get basic info from ctx for future use
    let cmd_name = ctx.command().name.as_str();

    let user_id = ctx.author().id.to_string();

    info!("{}", cmd_name);

    let data = ctx.data();
    let discord = ctx.discord();

    // Verify staff first
    let is_staff = crate::_checks::is_any_staff(ctx).await.unwrap_or_else(|e| {
        error!("{}", e);
        false
    });
    if !is_staff {
        // Check if awaiting staff role in main server
        let main_server = std::env::var("MAIN_SERVER").unwrap().parse::<u64>().unwrap();

        let member = discord.cache.member(main_server, ctx.author().id);

        if member.is_none() {
            info!("Member not found in main server");
            return Ok(true);
        }

        let member = member.unwrap();

        let awaiting_role = std::env::var("AWAITING_STAFF_ROLE").unwrap().parse::<u64>().unwrap();

        if !member.roles.contains(&RoleId(awaiting_role)) {
            info!("User is not awaiting staff role");
            return Ok(true);
        } 

        info!("User is awaiting staff role, adding staff perms and removing old onboarding state for the purpose of onboarding");

        sqlx::query!(
            "UPDATE users SET staff = true WHERE user_id = $1",
            user_id
        )
        .execute(&data.pool)
        .await?;

        sqlx::query!(
            "UPDATE users SET staff_onboard_state = 'pending' WHERE user_id = $1 AND staff_onboard_state = 'complete'",
            user_id
        )
        .execute(&data.pool)
        .await?;
    }

    // Reset old onboards
    sqlx::query!(
        "UPDATE users SET staff_onboard_state = 'pending', staff_onboard_last_start_time = NOW() WHERE staff_onboard_state = 'complete' AND staff = true AND NOW() - staff_onboard_last_start_time > interval '1 month'"
    )
    .execute(&data.pool)
    .await?;

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

    // Must be mut so we can change it under some cases, we use a second let to create a let binding
    let mut onboard_state = onboard_state.as_str();

    let onboarded = sqlx::query!(
        "SELECT staff_onboarded, staff_onboard_last_start_time FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(&data.pool)
    .await?;

    // Onboarding is complete, exit early
    if onboard_state == "complete" {
        return Ok(true);
    }

    if onboard_state == "pending-manager-review" {
        if cmd_name == "queue" {
            return Ok(true)
        }

        ctx.say(
            "Your onboarding request is pending manager review. Please wait until it is approved.",
        )
        .await?;
        return Ok(false);
    }

    if onboard_state == "denied" {
        if cmd_name == "queue" {
            return Ok(true)
        }

        ctx.say(
            "Your onboarding request was denied. Please contact a manager if you believe this was a mistake.",
        )
        .await?;
        return Ok(false);
    }

    if onboarded.staff_onboard_last_start_time.is_none() {
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

    if onboard_state == "pending" {
        // Set macro_time (when the onboarding is first started by the user)
        sqlx::query!(
            "UPDATE users SET staff_onboard_macro_time = NOW() WHERE user_id = $1", 
            user_id
        )
        .execute(&data.pool)
        .await?;
    }

    let cur_guild = ctx.guild().unwrap().name;

    if cur_guild.to_lowercase() != user_id.to_lowercase() {
        ctx.say("Creating new onboarding server for you!").await?;

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
                if name.to_lowercase() == user_id.to_lowercase() {
                    // Try to find a channel named readme
                    let mut channel = None;

                    for (_, chan) in guild.channels(&discord).await? {
                        if chan.name() == "readme" {
                            channel = Some(chan);
                            break;
                        }
                    }

                    if channel.is_none() {
                        // Delete guild and start over
                        crate::_utils::delete_leave_guild(&discord.http, &discord.cache, *guild).await;
                        continue;
                    }

                    let channel = channel.unwrap();

                    // Create new invite
                    let invite = channel
                        .create_invite(&discord, |i| {
                            i.max_age(0).max_uses(0).temporary(false).unique(true)
                        })
                        .await?;

                    // Send invite
                    ctx.send(|m| {
                        m.embed(|e| {
                            e.title("Onboarding Server")
                                .description("Click the link below to join the onboarding server.")
                                .color(0x00ff00)
                        })
                        .components(|c| {
                            c.create_action_row(|r| {
                                r.create_button(|b| {
                                    b.label("Join Onboarding Server")
                                        .style(serenity::ButtonStyle::Link)
                                        .url(&invite.url())
                                })
                            })
                        })
                    }).await?;

                    found = true;
                }
            }
        }

        if !found {
            let map = json!({
                "name": user_id,
            });

            let new_guild = discord.http.create_guild(&map).await?;

            let readme = new_guild
                .create_channel(&discord, |c| {
                    c.name("readme-")
                        .kind(serenity::model::channel::ChannelType::Text)
                })
                .await?;

            readme.say(&discord, "
Welcome to your onboarding server! Please read the following:

1. To start onboarding, run ``ibb!onboard`` in the #general channel.
2. There is a 1 hour time limit for onboarding. If you exceed this time limit, you will have to start over. You can extend this limit by progressing through onboarding.
")
                .await?;

            // Create a invite
            let invite = readme
                .create_invite(&discord, |i| {
                    i.max_age(0).max_uses(0).temporary(false).unique(true)
                })
                .await?;

            // Send invite
            ctx.send(|m| {
                m.embed(|e| {
                    e.title("Onboarding Server")
                        .description("Click the link below to join the onboarding server, then run ``ibb!onboard``.")
                        .color(0x00ff00)
                })
                .components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.label("Join Onboarding Server")
                                .style(serenity::ButtonStyle::Link)
                                .url(&invite.url())
                        })
                    })
                })
            }).await?;

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
                // This means the user has joined the server for the first time, so we check command name, then create a role
                if cmd_name != "onboard" {
                    ctx.say(
                        "Did you follow the instructions. You're supposed to run the ``ibb!onboard`` command!",
                    )
                    .await?;
                    return Ok(false);
                }

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
            let member = ctx.author_member().await;

            let mut member = member.unwrap().into_owned();

            member.add_role(&discord, role_id.unwrap()).await?;

            ctx.say(
                format!(
                    "You will need to reinvite this bot to the server so that Discord can set the required scopes for this bot! Reinvite the bot using https://discord.com/oauth2/authorize?client_id={}&scope=bot%20applications.commands&permissions=8. Do this now, then run ``ibb!onboard`` to continue!",
                    ctx.discord().cache.current_user().id
                )
            ).await?;

            return Ok(false);
        }
    }

    // Allow users to see queue again
    match (onboard_state, cmd_name) {
        ("claimed-bot" | "testing-bot", "queue") => {
            onboard_state = "claimed-bot";
        },
        (_, "queue") => {
            onboard_state = "queue-step";
        },
        ("queue-step", "staffguide") => {
            // We are now in staff_onboard_state of staff-guide, set that
            sqlx::query!(
                "UPDATE users SET staff_onboard_state = 'staff-guide-viewed' WHERE user_id = $1",
                user_id
            )
            .execute(&data.pool)
            .await?;
            _handle_staff_guide(ctx, user_id.to_string()).await?;
            return Ok(false);
        },
        (_, _) => {}
    }

    let test_bot = std::env::var("TEST_BOT")?;
    let bot_page = std::env::var("BOT_PAGE")?;
    let current_user = ctx.discord().cache.current_user();

    if cmd_name == "claim" && reason != Some(&test_bot) {
        ctx.say("You can only claim the test bot at this time!")
            .await?;
        return Ok(false);
    }

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
            if cmd_name != "onboard" {
                ctx.say(
                    "Did you follow the instructions. You're supposed to run the ``ibb!onboard`` command!",
                )
                .await?;
                return Ok(false);
            }

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

            ctx.say(r#"Whoa there! Look at that! There's a new bot to review!!! Type ``/queue`` (or ``ibb!queue``) to see the queue
            
**PRO TIP:** This has a time limit of one hour. Progressing through onboarding or using testing commands properly will reset the timer. You will **not** be informed of when your time limit is close to expiry. Changing the name of the server will cause it to be *deleted*
            "#).await?;

            Ok(false)
        }
        "testing-bot" => {
            // Allow staff guide here
            if cmd_name == "staffguide" {
                _handle_staff_guide(ctx, user_id.to_string()).await?;
                return Ok(false);
            }

            if cmd_name != "approve" && cmd_name != "deny" {
                ctx.say(
                    "Now you need to approve or deny this bot using the ``/approve`` (or ``ibb!approve``) or the ``/deny`` (or ``ibb!deny``) command!",
                )
                .await?;
                return Ok(false);
            }

            // Get more information about this action by launching a modal using a button
            let mut msg = ctx.send(|m| {
                m.content("Are you sure that you truly wish to ".to_string() + cmd_name + " this test bot?  If so, click 'Survey' to launch the final onboarding survey.\n\n**If you do not see a button, you will need to rerun the command.**")
                .components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.custom_id("survey")
                            .label("Survey")
                            .style(serenity::ButtonStyle::Primary)
                        })
                        .create_button(|b| {
                            b.custom_id("cancel")
                            .label("Cancel")
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
                .timeout(Duration::from_secs(120))
                .await;

            if let Some(m) = &interaction {
                let id = &m.data.custom_id;

                msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press

                if id == "survey" {
                    // Create a new message with the survey modal in it (via the button click)
                    m.create_interaction_response(discord, |b| {
                        b.kind(serenity::InteractionResponseType::Modal)
                        .interaction_response_data(|d| {
                            d.custom_id("survey")
                            .title("Survey")
                            .content("Sent survey response model")
                            .components(|c| {
                                c.create_action_row(|r| {
                                    r.create_input_text(|it| {
                                        it.custom_id("analysis")
                                        .label("In-depth analysis of all commands")
                                        .placeholder("State your analysis of all commands. What would you do for each command if this was a real bot")
                                        .style(serenity::InputTextStyle::Paragraph)
                                    })
                                });

                                c.create_action_row(|r| {
                                    r.create_input_text(|it| {
                                        it.custom_id("thoughts")
                                        .label("Your thoughts on onboarding")
                                        .placeholder("What did you think of the onboarding system? Your feedback will help us improve our services")
                                        .style(serenity::InputTextStyle::Paragraph)
                                    })
                                });

                                c.create_action_row(|r| {
                                    r.create_input_text(|it| {
                                        it.custom_id("code")
                                        .label("Staff Verify Code")
                                        .placeholder("You can find this by running the staffguide command")
                                        .style(serenity::InputTextStyle::Short)
                                    })
                                })
                            })
                        })
                    }).await?;

                    // Wait for user to submit
                    let response = serenity::CollectModalInteraction::new(&discord.shard)
                        .author_id(m.user.id)
                        .await;

                    if response.is_none() {
                        ctx.say("You took too long to respond. Please try again.").await?;
                        return Ok(false);
                    }
                    
                    let response = response.unwrap();
                    
                    // Verify the code
                    let i_code = crate::_utils::modal_get(&response.data, "code").extract_single();

                    if i_code.is_none() {
                        response.create_interaction_response(&discord, |ir| ir.interaction_response_data(|d| {
                            d.content("You did not provide a code. Please try again.")
                        })).await?;
                        return Ok(false);
                    }

                    let i_code = i_code.unwrap();

                    let code = sqlx::query!(
                        "SELECT staff_onboard_session_code FROM users WHERE user_id = $1",
                        user_id
                    )
                    .fetch_one(&data.pool)
                    .await?;  
                    
                    let code = code.staff_onboard_session_code;

                    if code.is_none() {
                        response.create_interaction_response(&discord, |ir| ir.interaction_response_data(|d| {
                            d.content("SVSession has expired, rerun ``/staffguide`` (or ``ibb!staffguide``) to get a new verification code")
                        })).await?;
                        return Ok(false);
                    }

                    let code = code.unwrap();

                    let codesplit = code.split('@').collect::<Vec<&str>>();

                    if codesplit.len() != 2 {
                        response.create_interaction_response(&discord, |ir| ir.interaction_response_data(|d| {
                            d.content("SVSession has expired [invalid], rerun ``/staffguide`` (or ``ibb!staffguide``) to get a new verification code")
                        })).await?;
                        return Ok(false);
                    }        
                    
                    let time_nonce = codesplit[1];
                    let time_nonce = time_nonce.parse::<i64>();
                
                    if time_nonce.is_err() {
                        response.create_interaction_response(&discord, |ir| ir.interaction_response_data(|d| {
                            d.content("SVSession has expired [invalid], rerun ``/staffguide`` (or ``ibb!staffguide``) to get a new verification code")
                        })).await?;
                        return Ok(false);
                    }              
                    
                    let time_nonce = time_nonce.unwrap();

                    // Get current time and subtract from time_nonce
                    let now = chrono::Utc::now().timestamp();
                
                    if now - time_nonce > 3600 {
                        response.create_interaction_response(&discord, |ir| ir.interaction_response_data(|d| {
                            d.content("SVSession has expired [invalid], rerun ``/staffguide`` (or ``ibb!staffguide``) to get a new verification code")
                        })).await?;
                        return Ok(false);
                    }

                    let code_web = codesplit[0];

                    // Take last 37 characters
                    let mut code_upper = code_web.chars().skip(code_web.len() - 37).collect::<String>();

                    // Set index 2 and 19 to 'r' and 'x' respectively
                    code_upper.replace_range(2..3,"r");
                    code_upper.replace_range(19..20,"x");

                    // SHA-512 it using ring
                    let code_upper = code_upper.as_bytes();
                    let code_upper = ring::digest::digest(&ring::digest::SHA512, code_upper);
                    let code_upper = data_encoding::HEXLOWER.encode(code_upper.as_ref());

                    // Get last 6 characters
                    let code_upper = code_upper.chars().skip(code_upper.len() - 6).collect::<String>();
                    
                    info!("Wanted {} and user inputted {}", code_upper, i_code);

                    if code_upper != i_code {
                        response.create_interaction_response(&discord, |ir| ir.interaction_response_data(|d| {
                            d.content("Whoa there! You inputted the wrong verification code (hint: ``/staffguide`` or ``ibb!staffguide``)")
                        })).await?;
                        return Ok(false);
                    }

                    response.create_interaction_response(&discord, |ir| ir.interaction_response_data(|d| {
                        d.content("And the magic continues... Thank you for completing the staff onboarding process! You will be notified when you are approved. Please wait while I send your application to the staff team...")
                    })).await?;

                    // Create permanent invite to this server
                    let channel = ctx.guild_id().unwrap().create_channel(discord, |c| {
                        c.kind(serenity::ChannelType::Text)
                        .name("do-not-delete")
                        .topic("This is a temporary channel used to create a permanent invite to the server. DO NOT DELETE.")
                    }).await?;

                    let inv = channel
                        .create_invite(discord, |i| i.max_age(0).max_uses(0).unique(true))
                        .await?;

                    channel.say(
                        discord,
                        format!(
                            "
{}, please do not delete this channel *or* leave this server until your onboarding is approved!!! 
                            
This bot *will* now leave this server however you should not! Be prepared to send invites to this server if needed by Managers!", 
                            ctx.author().mention()
                        )
                    ).await?;

                    let mut analysis = crate::_utils::modal_get(&response.data, "analysis").extract_single();

                    if analysis.is_none() {
                        analysis = Some("None".to_string());
                    }

                    let analysis = analysis.unwrap();

                    let mut thoughts = crate::_utils::modal_get(&response.data, "thoughts").extract_single();
                    
                    if thoughts.is_none() {
                        thoughts = Some("None".to_string());
                    }

                    let thoughts = thoughts.unwrap();

                    let s_onboard = sqlx::query!(
                        "SELECT staff_onboarded, staff_onboard_macro_time FROM users WHERE user_id = $1",
                        user_id
                    )
                    .fetch_one(&data.pool)
                    .await?;            

                    let survey_modal = json!({
                        "analysis": analysis,
                        "thoughts": thoughts,
                        "invite": inv.url(),
                        "submit_ts": chrono::Utc::now().timestamp(),
                        "start_ts": s_onboard.staff_onboard_macro_time.unwrap_or_default().timestamp(),
                        "staff_onboarded_before": s_onboard.staff_onboarded,
                    });

                    let tok = libavacado::public::gen_random(32);

                    sqlx::query!("INSERT INTO onboard_data (user_id, onboard_code, data) VALUES ($1, $2, $3)", 
                        user_id, 
                        tok,
                        survey_modal
                    )
                    .execute(&data.pool)
                    .await?;

                    // Now transfer ownership to author
                    ctx.guild_id()
                        .unwrap()
                        .edit(discord, |e| e.owner(ctx.author().id))
                        .await?;

                    let onboard_channel_id =
                        ChannelId(std::env::var("ONBOARDING_CHANNEL")?.parse::<u64>()?);

                    onboard_channel_id.say(
                        &discord,
                        format!(
                            "**New onboarding attempt**\n\n**User ID:** {user_id}\n**Action taken:** {action}\n**Overall reason:** {reason}.\n**URL:** {url}",
                            user_id = user_id,
                            action = cmd_name,
                            reason = reason.unwrap_or_default(),
                            url = "https://seed.infinitybots.gg/sovngarde/onboard?tok=".to_string() + &tok
                        )
                    ).await?;

                    sqlx::query!(
                        "UPDATE users SET staff_onboard_state = 'pending-manager-review' WHERE user_id = $1",
                        user_id
                    )
                    .execute(&data.pool)
                    .await?;

                    ctx.guild_id().unwrap().leave(discord).await?;

                    return Ok(false);
                } else {
                    m.create_interaction_response(&discord, |ir| ir.interaction_response_data(|d| {
                        d.content("Cancelled")
                    })).await?;
                    return Ok(false);
                }
            }

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

                ctx.say(r#"
Great! As you can see, you have now claimed ``Ninja Bot``. 
                
Now test the bot as per the staff guide. Then run either ``/approve`` or ``/deny`` with your overall feeling of whether or not this bot should 
be approved or denied.

**Note that you will need to verify that you have read the staff guide when using ``/approve`` or ``/deny``.**
"#).await?;

                sqlx::query!(
                    "UPDATE users SET staff_onboard_state = 'testing-bot' WHERE user_id = $1",
                    user_id
                )
                .execute(&data.pool)
                .await?;
            } else if cmd_name == "staffguide" {
                _handle_staff_guide(ctx, user_id.to_string()).await?;
                return Ok(false);
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
        "staff-guide-viewed" | "staff-guide-viewed-reminded" => {
            if cmd_name == "claim" {
                let mut msg = ctx
                    .send(|m| {
                        m.embed(|e| {
                            e.title("Bot Already Claimed")
                            .description(format!(
                                "This bot is already claimed by <@{}>",
                                current_user.id
                            ))
                            .color(0xFF0000)
                        })
                        .components(|c| {
                            c.create_action_row(|r| {
                                r.create_button(|b| {
                                    b.custom_id("fclaim")
                                        .style(serenity::ButtonStyle::Primary)
                                        .label("Force Claim")
                                        .disabled(onboard_state != "staff-guide-viewed-reminded")
                                });
                                r.create_button(|b| {
                                    b.custom_id("remind")
                                        .style(serenity::ButtonStyle::Secondary)
                                        .label("Remind Reviewer")
                                        .disabled(onboard_state != "staff-guide-viewed")
                                })
                            })
                        })
                    })
                    .await?
                    .into_message()
                    .await?;
                
                if onboard_state != "staff-guide-viewed-reminded" {
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
                    _handle_staff_guide(ctx, user_id.to_string()).await?;
                    return Ok(false);            
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

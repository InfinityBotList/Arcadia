use crate::_checks as checks;
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
    if !crate::_onboarding::handle_onboarding(ctx, &ctx.author().id.0.to_string(), false, None)
        .await?
    {
        return Ok(());
    }

    Ok(())
}

/// Sends the staff guide in paginated form
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 10,
    category = "Testing"
)]
pub async fn staffguide(ctx: Context<'_>) -> Result<(), Error> {
    if !crate::_onboarding::handle_onboarding(ctx, &ctx.author().id.0.to_string(), false, None)
        .await?
    {
        return Ok(());
    }

    let msgs = vec![
        r#"
**Welcome to the Infinity Bot List Staff Team**

*All commands below are shown in slash command form however prefix commands are well supported as well in case slash commands don't work*

**Logging**

All staff actions are logged to our database. In the past, staff activity was a huge problem. We hope doing this will
allow us to better manage Infinity Bot List to make it a truly wonderful place.

**Your Role**

Being a Website Moderator is also being the forefront of Infinity Bot List.
You are the first interaction for people on the server. You also have one of
the utmost important jobs for a Bot List.... Approving and Denying bots.

**The Process**

The process of approving and denying bots at Infinity Bot List is a rather simple and
straightforward process.

When a bot gets added to the bot queue it'll show up in the #bot-logs
channel in the Infinity Bot List Server. Here you will get a ping (``@Website Moderators``) and you 
will be able to view the bot profile page (which you must do while testing a bot)."#,
        r#"

**Resubmissions**

Resubmitted means that for whatever reason the bot has been denied. Such bots can be resubmitted by the owner of 
the bot. (An example of this is ``Ninja Bot`` from the training sandbox)

During all bot approvals and denials, regardless of whether it is resubmitted or not, the owner/developer of the bot 
must be a member of Infinity Bot List. *Arcadia (our management bot) will not let you approve/deny the bot otherwise*

Head to Verification Center. In the #queue channel in Verification Center, you can then use ``/queue`` to get the
bots pending verification.

**Queue Order**

Please go in Queue order. A bot thats in #2 position should not be done before a bot that is in #1 position! This is to
ensure that everyone has their bot reviewed fairly. If you see a ticket of the form "Why was my bot not yet tested", please
be sure to check the queue order and then inform accordingly. As of now, this queue order is public (by running ``/queue``)

**Inviting the bot**

You can use the ``/invite`` command to get the invite to the bot based on its Bot ID, Name or Vanity. 

**Claim the bot**

To limit confusion amongst other Website Moderators, Infinity Bot List has a claim system. Using ``/claim`` 
avoids multiple mods testing the same bot. If it turns out that you cannot test it after you've claimed it 
(ex. Something in real life came up that'll take longer than 30 minutes), use ``/unclaim``. We want to avoid bots 
sitting claimed for days with no testing being done *which is also why the queue shows claimed bots as well*.

*One difference from v3 in claims is the addition of "Force Claim" and "Remind" in ``/claim``. "Force Claim" allows 
you to forcibly claim a bot when it is currently being reviewed by someone else*
"#,
        r#"
**Some Pointers**

∞ When testing the bot please ensure you are doing an in depth test. Not just a handful of commands. Also please keep
in mind:

∞ If the bot goes offline during testing please message the owner either directly or in the #bot-feedback channel 
in the main server. Ex: "Hello @Toxic Dev your bot is offline and I can't test it. Let me know when this is fixed so 
I can continue the test." Please also do this if the bot is online but unresponsive.

∞ Please refer to the #info channel on the Verification Center for rules of what's acceptable and what's not acceptable. 

∞ If you have any questions please ping @Staff Managers or @Head Staff Managers. No question is a stupid question and we are always ready to help.

∞ After testing is complete please *DO NOT REMOVE THE BOT FROM THE TESTING SERVER. ARCADIA WILL DO THIS FOR YOU ONCE YOU HAVE ADDED IT TO THE MAIN SERVER*

∞ **You may test bots on your own server if you ever wish to. This may be required by some bots (eg. ticket bots, antinuke bots)**
"#,
        r#"
**After Testing**

You can *either* use the panel or this bot to approve or deny the bot. Panel may lag behind in terms of features and checks
so it is recommended to use this bot.

Please note that the owner must be in main server to use approve/deny. *Once approved, be sure to add it to the main server as arcadia will kick the bot from testing server for you.*

**Commonly asked permissions**

∞ *Administrator* - Bots that require the Administrator permission *on the bot account* to run should be denied always (*but please still test ``Ninja Bot`` and give feedback on all commands and what you would do in the training sandbox*)

∞ *Manage Channel* - Ticket bots commonly require this. Always test the functionality of the bot to see if it does anything related to channels before denying.

**Resources**

Cheatsheet of some common staff responses (highly recommended to use this): https://temp.botlist.site/
"#,
    ];

    let ephemeral = if ctx.guild().is_some() {
        ctx.guild().unwrap().name != ctx.author().id.to_string()
    } else {
        true
    };

    for msg in msgs {
        ctx.send(|m| m.content(msg).ephemeral(ephemeral)).await?;
    }

    Ok(())
}

/// Checks the bot queue
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing")]
pub async fn queue(
    ctx: Context<'_>,
    #[description = "Whether to embed or not"] embed: Option<bool>,
) -> Result<(), Error> {
    let embed = embed.unwrap_or(true);

    if !crate::_onboarding::handle_onboarding(ctx, &ctx.author().id.0.to_string(), embed, None)
        .await?
    {
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
    if !crate::_onboarding::handle_onboarding(
        ctx,
        &ctx.author().id.0.to_string(),
        false,
        Some(&bot.user.id.to_string()),
    )
    .await?
    {
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
        bot.user.id.0.to_string()
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
            bot.user.id.0.to_string()
        )
        .execute(&data.pool)
        .await?;

        libavacado::staff::add_action_log(
            &data.pool,
            &bot.user.id.0.to_string(),
            &ctx.author().id.0.to_string(),
            "Claimed",
            "claim",
        )
        .await?;

        ctx.send(|m| {
            m.embed(|e| {
                e.title("Bot Claimed")
                    .description(format!("You have claimed {}", bot.user.name))
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
                        bot.user.id.0
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
                    &bot.user.id.0.to_string(),
                    &claimed_by,
                    "User reminder",
                    "reminder",
                )
                .await?;
                ctx.say(
                    format!(
                        "<@{claimed_by}>, did you forgot to finish testing <@{bot_id}>? This reminder has been recorded internally for staff activity tracking purposes!", 
                        claimed_by = claimed_by,
                        bot_id = bot.user.id.0
                    )
                ).await?;
            } else {
                // Force claim
                sqlx::query!(
                    "UPDATE bots SET claimed = true, last_claimed = NOW(), claimed_by = $1 WHERE bot_id = $2",
                    ctx.author().id.0.to_string(),
                    bot.user.id.0.to_string()
                )
                .execute(&data.pool)
                .await?;

                libavacado::staff::add_action_log(
                    &data.pool,
                    &bot.user.id.0.to_string(),
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
                                bot.user.id.0,
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
                    bot_id = bot.user.id.0
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
    let data = ctx.data();
    let discord = ctx.discord();

    if !crate::_onboarding::handle_onboarding(ctx, &ctx.author().id.0.to_string(), false, None)
        .await?
    {
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
        bot.user.id.0.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    // Get main owner
    let owner = UserId(claimed.owner.parse::<u64>()?);

    if claimed.claimed_by.is_none() || claimed.claimed_by.as_ref().unwrap().is_empty() {
        ctx.say(format!("<@{}> is not claimed", bot.user.id.0))
            .await?;
    } else {
        sqlx::query!(
            "UPDATE bots SET claimed_by = NULL, claimed = false WHERE bot_id = $1",
            bot.user.id.0.to_string()
        )
        .execute(&data.pool)
        .await?;

        libavacado::staff::add_action_log(
            &data.pool,
            &bot.user.id.0.to_string(),
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
                            bot.user.id.0
                        ))
                        .footer(|f| f.text("This is completely normal, don't worry!"))
                })
            })
            .await?;

        ctx.say(format!("You have unclaimed <@{}>", bot.user.id.0))
            .await?;
    }

    Ok(())
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
    if !crate::_onboarding::handle_onboarding(
        ctx,
        &ctx.author().id.0.to_string(),
        false,
        Some(&reason),
    )
    .await?
    {
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
    if !crate::_onboarding::handle_onboarding(
        ctx,
        &ctx.author().id.0.to_string(),
        false,
        Some(&reason),
    )
    .await?
    {
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

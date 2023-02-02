use std::num::NonZeroU64;
use std::time::Duration;

use log::info;
use poise::serenity_prelude::{
    ChannelId, CreateActionRow, CreateAttachment, CreateButton, CreateChannel, CreateEmbed,
    CreateEmbedFooter, CreateInputText, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateInvite, CreateMessage, CreateQuickModal, CreateWebhook,
    EditGuild, EditRole, ExecuteWebhook, Mentionable, Permissions, RoleId, UserId,
};

use poise::{serenity_prelude as serenity, CreateReply};
use serde_json::json;

#[derive(PartialEq)]
pub enum OnboardState {
    Pending,
    QueueRemind,
    QueueForceClaim,
    Claimed,
    PendingManagerReview,
    Denied,
    Completed,
}

impl OnboardState {
    pub fn as_str(&self) -> &str {
        match self {
            OnboardState::Pending => "pending",
            OnboardState::QueueRemind => "queue-remind",
            OnboardState::QueueForceClaim => "queue-force-claim",
            OnboardState::Claimed => "claimed",
            OnboardState::PendingManagerReview => "pending-manager-review",
            OnboardState::Denied => "denied",
            OnboardState::Completed => "completed",
        }
    }

    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            "pending" => Some(OnboardState::Pending),
            "queue-remind" => Some(OnboardState::QueueRemind),
            "queue-force-claim" => Some(OnboardState::QueueForceClaim),
            "claimed" => Some(OnboardState::Claimed),
            "pending-manager-review" => Some(OnboardState::PendingManagerReview),
            "denied" => Some(OnboardState::Denied),
            "completed" => Some(OnboardState::Completed),
            _ => None,
        }
    }

    pub fn queue_unclaim(&self) -> bool {
        match self {
            OnboardState::Pending => true,
            OnboardState::QueueRemind => true,
            OnboardState::QueueForceClaim => true,
            _ => false,
        }
    }

    pub fn queue_passthrough(&self) -> bool {
        match self {
            OnboardState::Pending => true,
            OnboardState::PendingManagerReview => true,
            OnboardState::Denied => true,
            OnboardState::Completed => true,
            _ => false,
        }
    }
}

/// Internal function to handle the special-cased staff_guide command
///
/// This internally creates a onboarding 'fragment' which is used to ensure that a user isn't peeping into someone elses staff verification code
///
/// This fragment is then used by sovngarde to fetch the full code and add it to the guide.
async fn _handle_staff_guide(ctx: crate::Context<'_>, user_id: String) -> Result<(), crate::Error> {
    // This is the onboard code user needs to input (random_string@CURRENT_TIME)
    let onboard_code =
        libavacado::public::gen_random(64) + "@" + &chrono::Utc::now().timestamp().to_string();

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
            r#"The staff guide can be found at {url}/staff/guide?svu={uid}@{ocf}. Please **do not** bookmark this page as the URL may change in the future
            
Thats a lot isn't it? I'm glad you're ready to take on your first challenge. To get started, **invite ``Ninja Bot`` using ``ibb!invite [ID]`` where [ID] is the ID from the ``queue`` command**, then claim ``Ninja Bot``!

**Note that during onboarding, the *5 digit staff verify code present somewhere in the guide* will be reset every time you run the ``staffguide`` command! Always use the latest command invocation for getting the code**
            "#,
            url = &libavacado::CONFIG.frontend_url,
            uid = user_id,
            ocf = onboard_fragment,
    )).await?;

    Ok(())
}

/// Tries to check if onboarding is required, returns ``false`` if command should stop
pub async fn handle_onboarding(
    ctx: crate::Context<'_>,
    embed: bool,
    reason: Option<&str>,
) -> Result<bool, crate::Error> {
    // Get basic info from ctx for future use
    let cmd_name = ctx.command().name.as_str();

    let user_id = ctx.author().id.to_string();

    let data = ctx.data();
    let discord = ctx.discord();

    // Verify staff first
    let is_staff = crate::_checks::is_any_staff(ctx)
        .await
        .unwrap_or_else(|_| false)
        || {
            let member = discord
                .cache
                .member(libavacado::CONFIG.servers.main, ctx.author().id);

            if let Some(member) = member {
                if !member
                    .roles
                    .contains(&RoleId(libavacado::CONFIG.roles.awaiting_staff))
                {
                    false
                } else {
                    true
                }
            } else {
                false
            }
        };

    if !is_staff {
        return Ok(true);
    }

    // Reset old onboards
    sqlx::query!(
        "UPDATE users SET staff_onboard_state = $1 WHERE staff = true AND NOW() - staff_onboard_last_start_time > interval '1 month'",
        OnboardState::Pending.as_str()
    )
    .execute(&data.pool)
    .await?;

    // Get onboard state (staff_onboard_state may be used later but is right now None and it will in the future be used to allow retaking of onboarding)
    let onboard_state = OnboardState::from_str(&{
        let res = sqlx::query!(
            "SELECT staff_onboard_state FROM users WHERE user_id = $1",
            user_id
        )
        .fetch_one(&data.pool)
        .await?;

        res.staff_onboard_state
    })
    .unwrap_or(OnboardState::Pending);

    if onboard_state == OnboardState::Completed {
        return Ok(true);
    }

    if cmd_name == "queue" {
        if onboard_state.queue_passthrough() {
            return Ok(true);
        }

        if onboard_state.queue_unclaim() {
            // Send queue unclaim message here and abort
            let desc = format!(
                "**{i}.** {name} ({bot_id}) [Unclaimed]\n**Note**: {ap_note}\n**Bot Page**: {bp}/bots/{bot_id}",
                i = 1,
                name = "Ninja Bot",
                bot_id = libavacado::CONFIG.test_bot,
                bp = libavacado::CONFIG.frontend_url,
                ap_note = "Please test me :heart:"
            );
            if embed {
                ctx.send(
                    CreateReply::default().embed(
                        CreateEmbed::default()
                            .title("Bot Queue (Sandbox Mode)")
                            .description(desc)
                            .footer(CreateEmbedFooter::new(
                                "Use ibb!invite or /invite to get the bots invite",
                            ))
                            .color(0xA020F0),
                    ),
                )
                .await?;
            } else {
                ctx.say(desc).await?;
            }
            ctx.say(r#"
    You can use the `/queue` command to see the list of bots pending verification that *you* need to review!

    As you can see, ``Ninja Bot`` is whats currently pending review in this training sandbox.

    But before we get to reviewing it, lets have a look at the staff guide. You can see the staff guide by using ``/staffguide`` (or ``ibb!staffguide``)"#).await?;
            return Ok(false);
        } else {
            let desc = format!(
                "**{i}.** {name} ({bot_id}) [Claimed by: {claimed_by} (<@{claimed_by}>)]\n**Note:** {ap_note}",
                i = 1,
                name = "Ninja Bot",
                bot_id = libavacado::CONFIG.test_bot,
                claimed_by = ctx.author().id.0,
                ap_note = "Please test me :heart:"
            );
            if embed {
                ctx.send(
                    CreateReply::default().embed(
                        CreateEmbed::default()
                            .title("Bot Queue (Sandbox Mode)")
                            .description(desc)
                            .footer(CreateEmbedFooter::new(
                                "Use ibb!invite or /invite to get the bots invite",
                            ))
                            .color(0xA020F0),
                    ),
                )
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
        }

        return Ok(false);
    }

    if onboard_state == OnboardState::PendingManagerReview {
        ctx.say(
            "Your onboarding request is pending manager review. Please wait until it is approved.",
        )
        .await?;
        return Ok(false);
    }

    if onboard_state == OnboardState::Denied {
        ctx.say(
            "Your onboarding request was denied. Please contact a manager if you believe this was a mistake.",
        )
        .await?;
        return Ok(false);
    }

    let onboarded = sqlx::query!(
        "SELECT staff_onboarded, staff_onboard_guild, staff_onboard_last_start_time FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(&data.pool)
    .await?;

    let onboard_guild = onboarded.staff_onboard_guild.unwrap_or_default();

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
            "UPDATE users SET staff_onboard_last_start_time = NOW(), staff_onboard_state = $1 WHERE user_id = $2",
            OnboardState::Pending.as_str(),
            user_id
        )
        .execute(&data.pool)
        .await?;

        ctx.say(
            "You exceeded the time limit (1 hour) for the previous onboarding attempt! Save any progress you have made and rerun ``ibb!onboard``.",
        )
        .await?;

        return Ok(false);
    }

    if onboard_state == OnboardState::Pending {
        // Set macro_time (when the onboarding is first started by the user)
        sqlx::query!(
            "UPDATE users SET staff_onboard_macro_time = NOW() WHERE user_id = $1",
            user_id
        )
        .execute(&data.pool)
        .await?;
    }

    let cur_guild = ctx.guild().unwrap().id;

    if cur_guild.to_string() != onboard_guild {
        ctx.say("Creating/finding an onboarding server for you!")
            .await?;

        sqlx::query!(
            "UPDATE users SET staff_onboard_last_start_time = NOW() WHERE user_id = $1",
            user_id
        )
        .execute(&data.pool)
        .await?;

        // Check for old onboarding server
        let id = if onboard_guild != "" {
            if let Some(guild) = discord.cache.guild(onboard_guild.parse::<NonZeroU64>()?) {
                Some(guild.id)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(guild) = id {
            let mut channel = None;
            for (_, chan) in guild.channels(&discord.http).await? {
                if chan.name() == "readme" {
                    channel = Some(chan);
                    break;
                }
            }

            if channel.is_none() {
                // Create a new readme channel
                let readme = guild
                    .create_channel(&discord, CreateChannel::new("readme"))
                    .await?;

                readme.say(&discord, r#"
Welcome to your onboarding server! Please read the following:

1. To start onboarding, run ``ibb!onboard`` in the #general channel.
2. There is a 1 hour time limit for onboarding. If you exceed this time limit, you will have to start over. You can extend this limit by progressing through onboarding.                        
                "#).await?;

                channel = Some(readme);
            }

            let channel = channel.unwrap();

            // Create DM invite
            let invite = CreateInvite::new()
                .max_age(0)
                .max_uses(0)
                .temporary(false)
                .unique(true);
            let dm_invite = channel.create_invite(&discord, invite).await?;

            // Create DM channel
            let user_id = UserId(user_id.parse::<NonZeroU64>().unwrap());

            let dm_channel = user_id.create_dm_channel(discord).await?;

            // Send invite in DM
            let msg = CreateMessage::new()
            .embed(
                CreateEmbed::default()
                .title("Onboarding Server")
                .description("Click the link below to join the onboarding server. **This link is private**")
                .color(0x00ff00)
            )
            .components(
                vec![
                    CreateActionRow::Buttons(
                        vec![
                            CreateButton::new_link(&dm_invite.url()).label("Join Onboarding Server")
                        ]
                    )
                ]
            );

            dm_channel.send_message(discord, msg).await?;

            return Ok(false);
        } else {
            // Create a new guild
            let map = json!({
                "name": user_id,
            });

            let guild = discord.http.create_guild(&map).await?;

            sqlx::query!(
                "UPDATE users SET staff_onboard_guild = $1 WHERE user_id = $2",
                guild.id.to_string(),
                user_id
            )
            .execute(&data.pool)
            .await?;

            // Create a new readme channel
            let readme = guild
                .create_channel(&discord, CreateChannel::new("readme"))
                .await?;

            readme.say(&discord, r#"
Welcome to your onboarding server! Please read the following:

1. To start onboarding, run ``ibb!onboard`` in the #general channel.
2. There is a 1 hour time limit for onboarding. If you exceed this time limit, you will have to start over. You can extend this limit by progressing through onboarding.                        
            "#).await?;

            // Create invite
            let invite = CreateInvite::new()
                .max_age(0)
                .max_uses(0)
                .temporary(false)
                .unique(true);
            let invite = readme.create_invite(&discord, invite).await?;

            // Create DM channel
            let user_id = UserId(user_id.parse::<NonZeroU64>().unwrap());

            let dm_channel = user_id.create_dm_channel(discord).await?;

            // Send invite in DM
            let msg = CreateMessage::new()
            .embed(
                CreateEmbed::default()
                .title("Onboarding Server")
                .description("Click the link below to join the onboarding server. **This link is private**")
                .color(0x00ff00)
            )
            .components(
                vec![
                    CreateActionRow::Buttons(
                        vec![
                            CreateButton::new_link(&invite.url()).label("Join Onboarding Server")
                        ]
                    )
                ]
            );

            dm_channel.send_message(discord, msg).await?;

            let channel = ChannelId(libavacado::CONFIG.channels.onboarding_channel);

            // Send invite
            let sm_invite_msg = CreateMessage::default()
            .embed(
                CreateEmbed::default()
                .title("Onboarding Server")
                .description(
                    format!(
                        "Click the link below to join the onboarding server if you want to as a staff manager do so.\n**User ID:** ``{}``",
                        user_id.mention()
                    ))
                .color(0x00ff00)
            )
            .components(
                vec![
                    CreateActionRow::Buttons(
                        vec![
                            CreateButton::new_link(&invite.url()).label("Join Onboarding Server")
                        ]
                    )
                ]
            );
            channel.send_message(discord, sm_invite_msg).await?;

            return Ok(false);
        }
    } else {
        // Check if user is admin
        let mut found = false;

        for member in ctx.guild().unwrap().members.iter() {
            // Resolve the users permissions
            if member.0 .0 == ctx.author().id.0 {
                let permissions = member.1.permissions(discord)?;
                if permissions.administrator() {
                    found = true;
                }
            }
        }

        if !found {
            // Check for admin role
            let mut found = false;

            let mut role_id: Option<RoleId> = None;

            for role in ctx.guild().unwrap().roles.iter() {
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
                let guild_id = ctx.guild().unwrap().id;
                let role = guild_id
                    .create_role(
                        &discord,
                        EditRole::new()
                            .name("Head Administrator")
                            .colour(0x00ff00)
                            .permissions(Permissions::ADMINISTRATOR)
                            .mentionable(true),
                    )
                    .await?;

                role_id = Some(role.id);
            }

            if role_id.is_none() {
                ctx.say("Failed to fetch admin role").await?;
                return Ok(false);
            }

            // Add admin perms
            let member = ctx.author_member().await;

            let mut member = member.unwrap().into_owned();

            member.add_role(&discord, role_id.unwrap()).await?;
        }
    }

    // Staff guide just needs to _handle_staff_guide called
    if cmd_name == "staffguide" {
        _handle_staff_guide(ctx, user_id).await?;
        return Ok(false);
    }

    // We don't implement unclaim
    if cmd_name == "unclaim" {
        return Ok(false);
    }

    if cmd_name == "claim" && reason != Some(&libavacado::CONFIG.test_bot.to_string()) {
        ctx.say("You can only claim the test bot at this time!")
            .await?;
        return Ok(false);
    }

    // Before matching, make sure 'Ninja Bot' is always pending
    sqlx::query!(
        "UPDATE bots SET type = 'testbot' WHERE bot_id = $1",
        libavacado::CONFIG.test_bot.to_string()
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

    let current_user_id = ctx.discord().cache.current_user().id;
    let current_user_name = ctx.discord().cache.current_user().name.clone();

    match onboard_state {
        OnboardState::Pending => {
            if cmd_name != "onboard" {
                ctx.say(
                    "Did you follow the instructions. You're supposed to run the ``ibb!onboard`` command!",
                )
                .await?;
                return Ok(false);
            }

            ctx.say("**Welcome to Infinity Bot List**\n\nSince you seem new to this place, how about a nice look arou-?").await?;

            ctx.send(
                CreateReply::new()
                .embed(
                    CreateEmbed::new()
                    .title("Bot Resubmitted")
                    .description(
                        format!(
                            "**Bot:** <@{bot_id}> ({bot_name})\n\n**Owner:** {owner_id} ({owner_name})\n\n**Bot Page:** {frontend_url}/bots/{bot_id}",
                            bot_id = libavacado::CONFIG.test_bot,
                            bot_name = "Ninja Bot",
                            owner_id = current_user_id.mention(),
                            owner_name = current_user_name,
                            frontend_url = libavacado::CONFIG.frontend_url,
                        )
                    )
                    .footer(CreateEmbedFooter::new("Are you ready to take on your first challenge, young padawan?"))
                    .color(0xA020F0)
                )
            ).await?;

            sqlx::query!(
                "UPDATE users SET staff_onboard_state = $2 WHERE user_id = $1",
                user_id,
                OnboardState::QueueRemind.as_str()
            )
            .execute(&data.pool)
            .await?;

            ctx.say(r#"Whoa there! Look at that! There's a new bot to review!!! Type ``/queue`` (or ``ibb!queue``) to see the queue
            
**PRO TIP:** This has a time limit of one hour. Progressing through onboarding or using testing commands properly will reset the timer. You will **not** be informed of when your time limit is close to expiry. Changing the name of the server will cause it to be *deleted*
            "#).await?;

            Ok(false)
        }
        OnboardState::QueueRemind => {
            if cmd_name != "claim" {
                ctx.say(
                    "Did you follow the instructions. You're supposed to run the ``ibb!claim`` command!",
                )
                .await?;
                return Ok(false);
            }
            let builder = CreateReply::default()
                .embed(
                    CreateEmbed::default()
                        .title("Bot Already Claimed")
                        .description(format!(
                            "This bot is already claimed by <@{}>",
                            current_user_id
                        ))
                        .color(0xFF0000),
                )
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new("fclaim")
                        .label("Force Claim")
                        .style(serenity::ButtonStyle::Danger),
                    CreateButton::new("remind")
                        .label("Remind Reviewer")
                        .style(serenity::ButtonStyle::Secondary),
                ])]);

            let mut msg = ctx.send(builder.clone()).await?.into_message().await?;

            ctx.say("Woah! This bot is already claimed by someone else. Its always best practice to first remind the bot so do that!").await?;

            let interaction = msg
                .component_interaction_collector(ctx.discord())
                .author_id(ctx.author().id)
                .collect_single()
                .await;

            msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![]))
                .await?; // remove buttons after button press

            if let Some(interaction) = interaction {
                if interaction.data.custom_id != "remind" {
                    ctx.say("First remind, then force claim").await?;
                    return Ok(false);
                }
            }

            ctx.say(
                format!(
                    "<@{claimed_by}>, did you forgot to finish testing <@{bot_id}>? This reminder has been recorded internally for staff activity tracking purposes!", 
                    claimed_by = current_user_id,
                    bot_id = libavacado::CONFIG.test_bot
                )
            ).await?;

            // Create a discord webhook
            let wh = ctx
                .channel_id()
                .create_webhook(
                    discord,
                    CreateWebhook::new("Frostpaw").avatar(
                        &CreateAttachment::url(
                            discord,
                            "https://cdn.infinitybots.xyz/images/png/onboarding-v4.png",
                        )
                        .await?,
                    ),
                )
                .await?;

            tokio::time::sleep(Duration::from_secs(3)).await;

            wh.execute(
                discord, 
                true, 
                ExecuteWebhook::default().content("Ack! sorry about that. I completely forgot about Ninja Bot due to personal issues, yknow?")
            ).await?;

            ctx.say("Great! With a real bot, things won't go this smoothly, but you can always remind people to test their bot! Now try claiming again, but this time use ``Force Claim``").await?;

            sqlx::query!(
                "UPDATE users SET staff_onboard_state = $2 WHERE user_id = $1",
                user_id,
                OnboardState::QueueForceClaim.as_str()
            )
            .execute(&data.pool)
            .await?;

            Ok(false)
        }
        OnboardState::QueueForceClaim => {
            if cmd_name != "claim" {
                ctx.say(
                    "Did you follow the instructions. You're supposed to run the ``ibb!claim`` command!",
                )
                .await?;
                return Ok(false);
            }
            let builder = CreateReply::default()
                .embed(
                    CreateEmbed::default()
                        .title("Bot Already Claimed")
                        .description(format!(
                            "This bot is already claimed by <@{}>",
                            current_user_id
                        ))
                        .color(0xFF0000),
                )
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new("fclaim")
                        .label("Force Claim")
                        .style(serenity::ButtonStyle::Danger),
                    CreateButton::new("remind")
                        .label("Remind Reviewer")
                        .style(serenity::ButtonStyle::Secondary),
                ])]);

            let mut msg = ctx.send(builder.clone()).await?.into_message().await?;

            let interaction = msg
                .component_interaction_collector(ctx.discord())
                .author_id(ctx.author().id)
                .collect_single()
                .await;

            msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![]))
                .await?; // remove buttons after button press

            if let Some(interaction) = interaction {
                if interaction.data.custom_id == "remind" {
                    ctx.say("You already reminded, now force claim it since no one is responding")
                        .await?;
                    return Ok(false);
                }
            }

            sqlx::query!(
                "UPDATE users SET staff_onboard_state = $2 WHERE user_id = $1",
                user_id,
                OnboardState::Claimed.as_str()
            )
            .execute(&data.pool)
            .await?;

            ctx.say(format!(
                "You have claimed <@{bot_id}> and the bot owner has been notified!",
                bot_id = libavacado::CONFIG.test_bot
            ))
            .await?;

            ctx.say(
                "Now try using ``/queue`` (or ``ibb!queue``) to see what the queue looks like now!",
            )
            .await?;

            Ok(false)
        }
        OnboardState::Claimed => {
            if cmd_name != "approve" && cmd_name != "deny" {
                ctx.say(
                    "Now you need to approve or deny this bot using the ``/approve`` (or ``ibb!approve``) or the ``/deny`` (or ``ibb!deny``) command!",
                )
                .await?;
                return Ok(false);
            }

            // Get more information about this action by launching a modal using a button
            let builder = CreateReply::default()
            .content("Are you sure that you truly wish to ".to_string() + cmd_name + " this test bot?  If so, click 'Survey' to launch the final onboarding survey.\n\n**If you do not see a button, you will need to rerun the command.**")
            .components(
                vec![
                    CreateActionRow::Buttons(
                        vec![
                            CreateButton::new("survey")
                            .label("Survey")
                            .style(serenity::ButtonStyle::Primary),
                            CreateButton::new("cancel")
                            .label("Cancel")
                            .style(serenity::ButtonStyle::Danger)
                        ]
                    )
                ]
            );

            let mut msg = ctx.send(builder.clone()).await?.into_message().await?;

            let interaction = msg
                .component_interaction_collector(ctx.discord())
                .author_id(ctx.author().id)
                .timeout(Duration::from_secs(120))
                .collect_single()
                .await;

            if let Some(m) = &interaction {
                let id = &m.data.custom_id;

                msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![]))
                    .await?; // remove buttons after button press

                if id == "survey" {
                    // Create a new message with the survey modal in it (via the button click)
                    let qm = m.quick_modal(
                        discord, 
                        CreateQuickModal::new("Onboarding Survey")
                        .field(
                            CreateInputText::new(
                                serenity::InputTextStyle::Paragraph,
                                "In-depth analysis of all commands",
                                "analysis"   
                            )
                            .placeholder("State your analysis of all commands. What would you do for each command if this was a real bot")
                            .required(true)
                        )
                        .field(
                            CreateInputText::new(
                                serenity::InputTextStyle::Paragraph,
                                "Your thoughts on onboarding",
                                "thoughts",
                            )
                            .placeholder("What did you think of the onboarding system? Your feedback will help us improve our services")
                            .required(true)
                        )
                        .field(
                            CreateInputText::new(
                            serenity::InputTextStyle::Short,
                            "Staff Verify Code",
                            "code",
                            )
                            .placeholder("You can find this by running the staffguide command")
                            .required(true)
                        )
                        .timeout(Duration::from_secs(300))
                    ).await?;

                    if qm.is_none() {
                        ctx.say("You took too long to respond. Please try again")
                            .await?;
                        return Ok(false);
                    }

                    let qm = qm.unwrap();
                    let inputs = qm.inputs;

                    let (analysis, thoughts, i_code) = (&inputs[0], &inputs[1], &inputs[2]);

                    // Verify the code

                    let i_code = i_code.replace(' ', "");

                    let code = sqlx::query!(
                        "SELECT staff_onboard_session_code FROM users WHERE user_id = $1",
                        user_id
                    )
                    .fetch_one(&data.pool)
                    .await?;

                    let code = code.staff_onboard_session_code;

                    if code.is_none() {
                        qm.interaction.create_response(&discord, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                            .content("SVSession has expired, rerun ``/staffguide`` (or ``ibb!staffguide``) to get a new verification code")
                        )).await?;
                        return Ok(false);
                    }

                    let code = code.unwrap();

                    let codesplit = code.split('@').collect::<Vec<&str>>();

                    if codesplit.len() != 2 {
                        qm.interaction.create_response(&discord, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                            .content("SVSession has expired [internal codesplit error], rerun ``/staffguide`` (or ``ibb!staffguide``) to get a new verification code")
                        )).await?;
                        return Ok(false);
                    }

                    let time_nonce = codesplit[1];
                    let time_nonce = time_nonce.parse::<i64>();

                    if time_nonce.is_err() {
                        qm.interaction.create_response(&discord, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                            .content("SVSession has expired [internal error], rerun ``/staffguide`` (or ``ibb!staffguide``) to get a new verification code")
                        )).await?;
                        return Ok(false);
                    }

                    let time_nonce = time_nonce.unwrap();

                    // Get current time and subtract from time_nonce
                    let now = chrono::Utc::now().timestamp();

                    if now - time_nonce > 3600 {
                        qm.interaction.create_response(&discord, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                            .content("SVSession has expired [time nonce expiry], rerun ``/staffguide`` (or ``ibb!staffguide``) to get a new verification code")
                        )).await?;
                        return Ok(false);
                    }

                    let code_web = codesplit[0];

                    // Take last 37 characters
                    let mut code_upper = code_web
                        .chars()
                        .skip(code_web.len() - 37)
                        .collect::<String>();

                    // Set index 2 and 19 to 'r' and 'x' respectively
                    code_upper.replace_range(2..3, "r");
                    code_upper.replace_range(19..20, "x");

                    // SHA-512 it using ring
                    let code_upper = code_upper.as_bytes();
                    let code_upper = ring::digest::digest(&ring::digest::SHA512, code_upper);
                    let code_upper = data_encoding::HEXLOWER.encode(code_upper.as_ref());

                    // Get last 6 characters
                    let code_upper = code_upper
                        .chars()
                        .skip(code_upper.len() - 6)
                        .collect::<String>();

                    info!("Wanted {} and user inputted {}", code_upper, code);

                    if code_upper != i_code {
                        qm.interaction.create_response(&discord, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                            .content("Whoa there! You inputted the wrong verification code (hint: ``/staffguide`` or ``ibb!staffguide``)")
                        )).await?;
                        return Ok(false);
                    }

                    qm.interaction.create_response(&discord, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                        .content("And the magic continues... Thank you for completing the staff onboarding process! You will be notified when you are approved. Please wait while I send your application to the staff team...")
                    )).await?;

                    // Create permanent invite to this server
                    let channel = ctx.guild_id().unwrap().create_channel(
                        discord, 
                        CreateChannel::new("do-not-delete")
                        .topic("This is a temporary channel used to create a permanent invite to the server. DO NOT DELETE.")
                    ).await?;

                    let invite = channel
                        .create_invite(
                            discord,
                            CreateInvite::default()
                                .max_age(0)
                                .max_uses(0)
                                .temporary(false)
                                .unique(true),
                        )
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

                    let s_onboard = sqlx::query!(
                        "SELECT staff_onboarded, staff_onboard_macro_time FROM users WHERE user_id = $1",
                        user_id
                    )
                    .fetch_one(&data.pool)
                    .await?;

                    let survey_modal = json!({
                        "analysis": analysis,
                        "thoughts": thoughts,
                        "invite": invite.url(),
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
                    let edit = EditGuild::default().owner(ctx.author().id);
                    ctx.guild_id().unwrap().edit(discord, edit).await?;

                    let onboard_channel_id =
                        ChannelId(libavacado::CONFIG.channels.onboarding_channel);

                    onboard_channel_id.say(
                        &discord,
                        format!(
                            "**New onboarding attempt**\n\n**User ID:** {user_id}\n**Action taken:** {action}\n**Overall reason:** {reason}.\n**URL:** {url}",
                            user_id = user_id,
                            action = cmd_name,
                            reason = reason.unwrap_or_default(),
                            url = "https://ptb.botlist.app/staff/onboardresp/".to_string() + &tok
                        )
                    ).await?;

                    sqlx::query!(
                        "UPDATE users SET staff_onboard_state = $2 WHERE user_id = $1",
                        user_id,
                        OnboardState::PendingManagerReview.as_str()
                    )
                    .execute(&data.pool)
                    .await?;

                    ctx.guild_id().unwrap().leave(discord).await?;

                    return Ok(false);
                } else {
                    m.create_response(
                        &discord,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default().content("Cancelled"),
                        ),
                    )
                    .await?;
                    return Ok(false);
                }
            }

            Ok(false)
        }
        _ => {
            ctx.say("Unknown onboarding state:".to_string() + onboard_state.as_str())
                .await?;
            Ok(false)
        }
    }
}

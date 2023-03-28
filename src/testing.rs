use crate::{checks, config, impls};
use futures_util::StreamExt;
use log::info;
use poise::serenity_prelude::{
    ChannelId, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateMessage, User,
};
use poise::{serenity_prelude as serenity, CreateReply};
use serde::Serialize;
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

/// Sends the staff guide link
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 10,
    category = "Testing"
)]
pub async fn staffguide(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say(
        format!(
            "The staff guide can be found at {}/staff/guide. Please **do not** bookmark this page as the URL may change in the future",
            config::CONFIG.frontend_url
    )).await?;

    Ok(())
}

struct InternalQueueBot {
    index: usize,
    total_bots: usize,
    bot_id: String,
    queue_name: String,
    text_msg: bool,
    claimed_by: Option<String>,
    approval_note: String,
    short: String,
    owner: String,
    invite: String,
}

fn _queue_bot(qb: InternalQueueBot) -> CreateReply {
    let reply = if qb.text_msg {
        let text_msg = format!("**{name} [{c_bot}/{bot_len}]**\n**ID:** {id}\n**Claimed by:** {claimed_by}\n**Approval note:** {approve_note}\n**Short:** {short}\n**Queue name:** {name}\n**Owner:** {owner}\n**Invite:** {invite}", 
            name = qb.queue_name,
            c_bot = qb.index + 1,
            bot_len = qb.total_bots,
            id = qb.bot_id,
            claimed_by = qb.claimed_by.unwrap_or_else(|| "*You are free to test this bot. It is not claimed*".to_string()), 
            approve_note = qb.approval_note,
            short = qb.short,
            owner = qb.owner,
            invite = qb.invite
        );

        CreateReply::default().content(text_msg)
    } else {
        let embed = serenity::CreateEmbed::default()
            .title(format!(
                "{name} {c_bot}/{bot_len}",
                name = qb.queue_name,
                c_bot = qb.index + 1,
                bot_len = qb.total_bots
            ))
            .field("ID", qb.bot_id.clone(), false)
            .field("Short", qb.short, false)
            .field("Owner", qb.owner, false)
            .field(
                "Claimed by",
                qb.claimed_by.unwrap_or_else(|| {
                    "*You are free to test this bot. It is not claimed*".to_string()
                }),
                false,
            )
            .field("Approval note", qb.approval_note, true)
            .field("Queue name", qb.queue_name, true)
            .field("Invite", format!("[Invite Bot]({})", qb.invite), true);

        CreateReply::default().embed(embed)
    };

    reply.components(vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new("q:prev")
                .label("Previous")
                .style(serenity::ButtonStyle::Primary)
                .disabled(qb.index == 0),
            CreateButton::new("q:cancel")
                .label("Cancel")
                .style(serenity::ButtonStyle::Danger),
            CreateButton::new("q:next")
                .label("Next")
                .style(serenity::ButtonStyle::Primary)
                .disabled(qb.index >= qb.total_bots - 1),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new_link(qb.invite).label("Invite"),
            CreateButton::new_link(config::CONFIG.frontend_url.clone() + "/bots/" + &qb.bot_id)
                .label("View Page"),
        ]),
    ])
}

/// Checks the bot queue
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing")]
pub async fn queue(
    ctx: Context<'_>,
    #[description = "Whether to embed or not"] embed: Option<bool>,
) -> Result<(), Error> {
    let embed = embed.unwrap_or(true);

    let data = ctx.data();

    let bots = sqlx::query!(
        "SELECT claimed_by, bot_id, approval_note, short, queue_name, invite FROM bots WHERE type = 'pending' ORDER BY created_at ASC",
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
    let bot = &bots[current_bot];

    let bot_owner = crate::impls::utils::resolve_ping_user(&bot.bot_id, &data.pool).await?;

    let mut msg = ctx
        .send(_queue_bot(InternalQueueBot {
            index: current_bot,
            total_bots: bot_len,
            bot_id: bot.bot_id.clone(),
            queue_name: bot.queue_name.clone(),
            text_msg: !embed,
            claimed_by: bot.claimed_by.clone(),
            approval_note: bot.approval_note.clone(),
            short: bot.short.clone(),
            owner: bot_owner,
            invite: bot.invite.clone(),
        }))
        .await?
        .into_message()
        .await?;

    let mut interaction = msg
        .await_component_interactions(ctx.discord())
        .author_id(ctx.author().id)
        .timeout(Duration::from_secs(120))
        .stream();

    while let Some(item) = interaction.next().await {
        item.defer(&ctx.discord()).await?;

        let id = &item.data.custom_id;

        info!("Received interaction: {}", id);

        if id == "q:cancel" {
            item.delete_response(ctx.discord()).await?;
            return Ok(());
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

        let bot = &bots[current_bot];

        let bot_owner = crate::impls::utils::resolve_ping_user(&bot.bot_id, &data.pool).await?;

        msg.edit(
            ctx,
            _queue_bot(InternalQueueBot {
                index: current_bot,
                total_bots: bot_len,
                bot_id: bot.bot_id.clone(),
                queue_name: bot.queue_name.clone(),
                text_msg: !embed,
                claimed_by: bot.claimed_by.clone(),
                approval_note: bot.approval_note.clone(),
                short: bot.short.clone(),
                owner: bot_owner,
                invite: bot.invite.clone(),
            })
            .to_prefix_edit(),
        )
        .await?;
    }

    Ok(())
}

/// Claims a bot
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing", check = "checks::is_staff")]
pub async fn claim(
    ctx: Context<'_>,
    #[description = "The bot you wish to claim"] bot: User,
) -> Result<(), Error> {
    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    // Check if its claimed by someone
    let data = ctx.data();
    let discord = ctx.discord();

    let claimed = sqlx::query!(
        "SELECT type, claimed_by FROM bots WHERE bot_id = $1",
        bot.id.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    if claimed.r#type != "pending" {
        return Err("This bot is not pending review".into());
    }

    if claimed.r#type == "testbot" {
        return Err("This bot is a test bot".into());
    }

    let bot_owner = crate::impls::utils::resolve_ping_user(&bot.id.to_string(), &data.pool).await?;

    if claimed.claimed_by.is_none() {
        // Claim it
        sqlx::query!(
            "UPDATE bots SET last_claimed = NOW(), claimed_by = $1 WHERE bot_id = $2",
            ctx.author().id.0.to_string(),
            bot.id.to_string()
        )
        .execute(&data.pool)
        .await?;

        impls::actions::add_action_log(
            &data.pool,
            &bot.id.to_string(),
            &ctx.author().id.0.to_string(),
            "Claimed",
            "claim",
        )
        .await?;

        let msg = CreateReply::default().embed(
            CreateEmbed::default()
                .title("Bot Claimed")
                .description(format!("You have claimed <@{}>", bot.id))
                .footer(CreateEmbedFooter::new(
                    "Use ibb!invite or /invite to get the bots invite",
                )),
        );

        ctx.send(msg).await?;

        let msg = CreateMessage::default()
            .content(format!("<@{}>", bot_owner))
            .embed(
                CreateEmbed::default()
                    .title("Bot Claimed!")
                    .description(format!(
                        "<@{}> has claimed <@{}>",
                        ctx.author().id.0,
                        bot.id
                    ))
                    .footer(CreateEmbedFooter::new(
                        "This is completely normal, don't worry!",
                    )),
            );

        ChannelId(crate::config::CONFIG.channels.mod_logs)
            .send_message(discord, msg)
            .await?;
    } else {
        let builder = CreateReply::default()
            .embed(
                CreateEmbed::default()
                    .title("Bot Already Claimed")
                    .description(format!(
                        "This bot is already claimed by <@{}>",
                        claimed.claimed_by.as_ref().ok_or("No claimed_by")?
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
            .await_component_interaction(ctx.discord())
            .author_id(ctx.author().id)
            .await;

        msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![]))
            .await?; // remove buttons after button press

        if let Some(m) = &interaction {
            let id = &m.data.custom_id;

            let claimed_by = claimed.claimed_by.unwrap();

            if id == "remind" {
                impls::actions::add_action_log(
                    &data.pool,
                    &bot.id.to_string(),
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
                    "UPDATE bots SET last_claimed = NOW(), claimed_by = $1 WHERE bot_id = $2",
                    ctx.author().id.0.to_string(),
                    bot.id.to_string()
                )
                .execute(&data.pool)
                .await?;

                impls::actions::add_action_log(
                    &data.pool,
                    &bot.id.to_string(),
                    &ctx.author().id.0.to_string(),
                    "Force claim since previous staff did not finish reviewing bot",
                    "claim",
                )
                .await?;

                let msg = CreateMessage::default()
                    .content(format!("<@{}>", bot_owner))
                    .embed(
                        CreateEmbed::default()
                            .title("Bot Reclaimed!")
                            .description(format!(
                                "<@{}> has reclaimed <@{}> from <@{}>",
                                ctx.author().id.0,
                                bot.id,
                                claimed_by
                            ))
                            .footer(CreateEmbedFooter::new(
                                "This is completely normal, don't worry!",
                            )),
                    );

                ChannelId(crate::config::CONFIG.channels.mod_logs)
                    .send_message(discord, msg)
                    .await?;

                ctx.say(format!(
                    "You have claimed <@{bot_id}> and the bot owner has been notified!",
                    bot_id = bot.id
                ))
                .await?;
            }
        }
    }

    Ok(())
}

/// Unclaims a bot
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing", check = "checks::is_staff")]
pub async fn unclaim(
    ctx: Context<'_>,
    #[description = "The bot you wish to unclaim"] bot: serenity::User,
    #[description = "Reason for unclaiming"] reason: String,
) -> Result<(), Error> {
    let data = ctx.data();
    let discord = ctx.discord();

    if bot.id.0 == config::CONFIG.test_bot {
        return Err("You cannot unclaim the test bot!".into());
    }

    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let claimed = sqlx::query!(
        "SELECT type, claimed_by, owner FROM bots WHERE bot_id = $1",
        bot.id.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    if claimed.r#type != "pending" {
        return Err("This bot is not pending review".into());
    }

    let bot_owner = crate::impls::utils::resolve_ping_user(&bot.id.to_string(), &data.pool).await?;

    if claimed.claimed_by.is_none() {
        ctx.say(format!("<@{}> is not claimed", bot.id.0)).await?;
    } else {
        sqlx::query!(
            "UPDATE bots SET claimed_by = NULL, type = 'pending' WHERE bot_id = $1",
            bot.id.0.to_string()
        )
        .execute(&data.pool)
        .await?;

        impls::actions::add_action_log(
            &data.pool,
            &bot.id.to_string(),
            &ctx.author().id.to_string(),
            &reason,
            "unclaim",
        )
        .await?;

        let msg = CreateMessage::new()
            .content(format!("<@{}>", bot_owner))
            .embed(
                CreateEmbed::new()
                    .title("Bot Unclaimed!")
                    .description(format!(
                        "<@{}> has unclaimed <@{}>",
                        ctx.author().id.0,
                        bot.id.0
                    ))
                    .field("Reason", reason, false)
                    .footer(CreateEmbedFooter::new(
                        "This is completely normal, don't worry!",
                    )),
            );

        ChannelId(crate::config::CONFIG.channels.mod_logs)
            .send_message(discord, msg)
            .await?;

        ctx.say(format!("You have unclaimed <@{}>", bot.id.0))
            .await?;
    }

    Ok(())
}

/// Approves a bot
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing", check = "checks::is_staff")]
pub async fn approve(
    ctx: Context<'_>,
    #[description = "The bot you wish to approve"] bot: serenity::Member,
    #[description = "The reason for approval"] reason: String,
) -> Result<(), Error> {
    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let data = ctx.data();
    let resp = impls::actions::approve_bot(
        &data.cache_http,
        &data.pool,
        &bot.user.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say(
        format!("Approved bot\nNext invite it to the main server and it should be removed from this server: {}", resp)
    ).await?;

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
    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let data = ctx.data();
    impls::actions::deny_bot(
        &data.cache_http,
        &data.pool,
        &bot.id.to_string(),
        &ctx.author().id.to_string(),
        &reason,
    )
    .await?;

    ctx.say("Denied bot").await?;

    Ok(())
}

use crate::impls::dovewing::DovewingSource;
use crate::impls::target_types::TargetType;
use crate::{checks, config};
use futures_util::StreamExt;
use log::info;
use poise::serenity_prelude::{CreateActionRow, CreateButton, CreateEmbed, User};
use poise::{serenity_prelude as serenity, CreateReply};
use serde_json::json;
use std::time::Duration;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Gets the invite to a bot
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing")]
pub async fn invite(
    ctx: Context<'_>,
    #[description = "The invite to the bot"] bot: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let invite_data = sqlx::query!(
        "SELECT invite FROM bots WHERE bot_id = $1 ORDER BY created_at DESC LIMIT 1",
        bot
    )
    .fetch_one(&data.pool)
    .await?;

    ctx.say(&format!("Invite: {}", invite_data.invite)).await?;
    Ok(())
}

/// Gets a safe invite to a bot
#[poise::command(prefix_command, slash_command, user_cooldown = 3, category = "Testing")]
pub async fn invitesafe(
    ctx: Context<'_>,
    #[description = "The invite to the bot"] bot: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let invite_data = sqlx::query!("SELECT client_id FROM bots WHERE bot_id = $1", bot)
        .fetch_one(&data.pool)
        .await?;

    ctx.say(
        format!(
            "https://discord.com/api/v10/oauth2/authorize?client_id={client_id}&permissions=0&scope=bot%20applications.commands&guild_id={guild_id}", 
            client_id = invite_data.client_id,
            guild_id = crate::config::CONFIG.servers.main
        )
    ).await?;
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

fn _queue_bot<'a>(qb: InternalQueueBot) -> CreateReply<'a> {
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
        "SELECT claimed_by, bot_id, approval_note, short, invite FROM bots WHERE type = 'pending' ORDER BY created_at ASC",
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

    let owners =
        crate::impls::utils::get_entity_managers(TargetType::Bot, &bot.bot_id, &data.pool).await?;

    let bot_partial: crate::impls::dovewing::PlatformUser = crate::impls::dovewing::get_platform_user(
        &data.pool,
        DovewingSource::Discord(crate::impls::cache::CacheHttpImpl::from_ctx(ctx.serenity_context())),
        &bot.bot_id,
    )
    .await?;

    let mut msg = ctx
        .send(_queue_bot(InternalQueueBot {
            index: current_bot,
            total_bots: bot_len,
            bot_id: bot.bot_id.clone(),
            queue_name: bot_partial.display_name,
            text_msg: !embed,
            claimed_by: bot.claimed_by.clone(),
            approval_note: bot.approval_note.clone(),
            short: bot.short.clone(),
            owner: owners.mention_users(),
            invite: bot.invite.clone(),
        }))
        .await?
        .into_message()
        .await?;

    let mut interaction = msg
        .await_component_interactions(ctx.serenity_context().shard.clone())
        .author_id(ctx.author().id)
        .timeout(Duration::from_secs(120))
        .stream();

    while let Some(item) = interaction.next().await {
        item.defer(&ctx.serenity_context().http).await?;

        let id = &item.data.custom_id;

        info!("Received interaction: {}", id);

        if id == "q:cancel" {
            item.delete_response(&ctx.serenity_context().http).await?;
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

        let owners =
            crate::impls::utils::get_entity_managers(TargetType::Bot, &bot.bot_id, &data.pool)
                .await?;

        let bot_partial = crate::impls::dovewing::get_platform_user(
            &data.pool,
            DovewingSource::Discord(crate::impls::cache::CacheHttpImpl::from_ctx(ctx.serenity_context())),
            &bot.bot_id,
        )
        .await?;

        msg.edit(
            ctx,
            _queue_bot(InternalQueueBot {
                index: current_bot,
                total_bots: bot_len,
                bot_id: bot.bot_id.clone(),
                queue_name: bot_partial.display_name,
                text_msg: !embed,
                claimed_by: bot.claimed_by.clone(),
                approval_note: bot.approval_note.clone(),
                short: bot.short.clone(),
                owner: owners.mention_users(),
                invite: bot.invite.clone(),
            })
            .to_prefix_edit(poise::serenity_prelude::EditMessage::default()),
        )
        .await?;
    }

    Ok(())
}

/// Claims a bot
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 3,
    category = "Testing",
    check = "checks::is_staff",
    check = "checks::needs_onboarding"
)]
pub async fn claim(
    ctx: Context<'_>,
    #[description = "The bot you wish to claim"] bot: User,
) -> Result<(), Error> {
    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    // Check if its claimed by someone
    let data = ctx.data();

    let claimed = sqlx::query!(
        "SELECT type, claimed_by FROM bots WHERE bot_id = $1",
        bot.id.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    if claimed.r#type != "pending" {
        return Err("This bot is not pending review".into());
    }

    let mut force = false;

    if let Some(claimed_by) = claimed.claimed_by {
        let builder = CreateReply::default()
            .embed(
                CreateEmbed::default()
                    .title("Bot Already Claimed")
                    .description(format!("This bot is already claimed by <@{}>", &claimed_by,))
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
            .await_component_interaction(ctx.serenity_context().shard.clone())
            .author_id(ctx.author().id)
            .await;

        msg.edit(
            ctx.serenity_context(),
            builder
                .to_prefix_edit(poise::serenity_prelude::EditMessage::default())
                .components(vec![]),
        )
        .await?; // remove buttons after button press

        if let Some(m) = &interaction {
            let id = &m.data.custom_id;

            if id == "remind" {
                sqlx::query!(
                    "INSERT INTO staff_general_logs (user_id, action, data) VALUES ($1, $2, $3)",
                    ctx.author().id.to_string(),
                    "claim_reminder",
                    json!({
                        "bot_id": bot.id.to_string(),
                        "claimed_by": claimed_by,
                    })
                )
                .execute(&ctx.data().pool)
                .await?;

                ctx.say(
                    format!(
                        "<@{claimed_by}>, did you forgot to finish testing <@{bot_id}>? This reminder has been recorded internally for staff activity tracking purposes!", 
                        claimed_by = claimed_by,
                        bot_id = bot.id
                    )
                ).await?;

                return Ok(());
            } else {
                force = true;
            }
        }
    }

    // Create a rpc call
    crate::rpc::core::RPCMethod::Claim {
        target_id: bot.id.to_string(),
        force,
    }
    .handle(crate::rpc::core::RPCHandle {
        pool: data.pool.clone(),
        cache_http: crate::impls::cache::CacheHttpImpl::from_ctx(ctx.serenity_context()),
        user_id: ctx.author().id.to_string(),
        target_type: TargetType::Bot,
    })
    .await?;

    ctx.say("Claimed bot successfully, the bot owner has been informed")
        .await?;

    Ok(())
}

/// Unclaims a bot
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 3,
    category = "Testing",
    check = "checks::is_staff",
    check = "checks::needs_onboarding"
)]
pub async fn unclaim(
    ctx: Context<'_>,
    #[description = "The bot you wish to unclaim"] bot: serenity::User,
    #[description = "Reason for unclaiming"] reason: String,
) -> Result<(), Error> {
    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let data = ctx.data();

    ctx.defer_or_broadcast().await?;

    crate::rpc::core::RPCMethod::Unclaim {
        target_id: bot.id.to_string(),
        reason: reason.clone(),
    }
    .handle(crate::rpc::core::RPCHandle {
        pool: data.pool.clone(),
        cache_http: crate::impls::cache::CacheHttpImpl::from_ctx(ctx.serenity_context()),
        user_id: ctx.author().id.to_string(),
        target_type: TargetType::Bot,
    })
    .await?;

    ctx.say("Unclaimed bot successfully!").await?;

    Ok(())
}

/// Approves a bot
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 3,
    category = "Testing",
    check = "checks::is_staff",
    check = "checks::needs_onboarding"
)]
pub async fn approve(
    ctx: Context<'_>,
    #[description = "The bot you wish to approve"] bot: serenity::Member,
    #[description = "The reason for approval"] reason: String,
) -> Result<(), Error> {
    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let data = ctx.data();

    ctx.defer_or_broadcast().await?;

    // Create a rpc call
    let res = crate::rpc::core::RPCMethod::Approve {
        target_id: bot.user.id.to_string(),
        reason: reason.clone(),
    }
    .handle(crate::rpc::core::RPCHandle {
        pool: data.pool.clone(),
        cache_http: crate::impls::cache::CacheHttpImpl::from_ctx(ctx.serenity_context()),
        user_id: ctx.author().id.to_string(),
        target_type: TargetType::Bot,
    })
    .await?;

    let content = res.content().ok_or("RPC did not return as expected???")?;

    ctx.say(
        format!("Approved bot!\nPlease invite the bot, to the Caching Server provided down below!\n{}", content)
    ).await?;

    Ok(())
}

/// Denies a bot
#[poise::command(
    prefix_command,
    slash_command,
    user_cooldown = 10,
    category = "Testing",
    check = "checks::is_staff",
    check = "checks::needs_onboarding"
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

    ctx.defer_or_broadcast().await?;

    crate::rpc::core::RPCMethod::Deny {
        target_id: bot.id.to_string(),
        reason: reason.clone(),
    }
    .handle(crate::rpc::core::RPCHandle {
        pool: data.pool.clone(),
        cache_http: crate::impls::cache::CacheHttpImpl::from_ctx(ctx.serenity_context()),
        user_id: ctx.author().id.to_string(),
        target_type: TargetType::Bot,
    })
    .await?;

    ctx.say("Denied bot").await?;

    Ok(())
}

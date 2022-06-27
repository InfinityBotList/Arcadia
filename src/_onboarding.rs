use log::info;
use poise::serenity_prelude::Mentionable;

/// Tries to check if onboarding is required, returns ``false`` if command should stop
pub async fn handle_onboarding(ctx: crate::Context<'_>, user_id: &str, set_onboard_state: Option<String>) -> Result<bool, crate::Error> {
    if !crate::checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let cmd_name = ctx.command().name;

    info!("{}", cmd_name);

    let data = ctx.data();

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
    if cmd_name == "queue" && onboard_state != "pending" && onboard_state != "complete" {
        onboard_state = "queue-step";
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
    } else if chrono::offset::Utc::now() - onboarded.staff_onboard_last_start_time.unwrap() > chrono::Duration::hours(1) {
        sqlx::query!(
            "UPDATE users SET staff_onboard_last_start_time = NOW(), staff_onboard_state = 'pending' WHERE user_id = $1",
            user_id
        )
        .execute(&data.pool)
        .await?;

        ctx.say("You exceeded the time limit (1 hour) for the previous onboarding attempt. Retrying...").await?;

        onboard_state = "pending";
    }

    match onboard_state {
        "pending" => {
            let test_bot = std::env::var("TEST_BOT")?;
            let bot_page = std::env::var("BOT_PAGE")?;
            
            ctx.say("**Welcome to Infinity Bot List**\n\nSince you seem new to this place, how about a nice look arou-?").await?;

            ctx.send(|m| {
                let current_user = ctx.discord().cache.current_user();

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

            Ok(false)
        },
        "queue-step" => {
            let test_bot = std::env::var("TEST_BOT")?;

            if cmd_name == "queue" {
                ctx.send(|m| {
                    m.embed(|e| {
                        e.title("Bot Queue (Sandbox Mode)")
                        .description("**1.** Ninja Bot (".to_string()+&test_bot+") [Unclaimed]")
                        .footer(|f| {
                            f.text("Use ibb!invite or /invite to get the bots invite")
                        })
                        .color(0xA020F0)
                    })
                }).await?;
                ctx.say(r#"
You can use the `/queue` command to see the list of bots pending verification that *you* need to review!

As you can see, ``Ninja Bot`` is whats currently pending review in this training sandbox.

But before we get to reviewing it, lets have a look at the staff guide. You can see the staff guide by using ``/staffguide`` (or ``ibb!staffguide``)"#).await?;
            } else {
                ctx.say("You can use the `/queue` command to see the list of bots pending verification that *you* need to review! Lets try that out?").await?;
            }

            Ok(false)
        },
        "staff-guide" => {
            Ok(true)
        },
        "staff-guide-viewed" => {
            Ok(true)
        },
        "complete" => Ok(true),
        _ => {
            ctx.say("Unknown onboarding state:".to_string() + onboard_state).await?;
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
            sqlx::query!(
                "UPDATE users SET staff_onboard_state = 'staff-guide' WHERE user_id = $1",
                ctx.author().id.to_string()
            )
            .execute(&data.pool)
            .await?;
            Ok(())
        },
        _ => Ok(())
    }
}
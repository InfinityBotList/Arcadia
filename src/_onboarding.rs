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

    let onboard_state = onboard_state.as_str();
    
    let _onboarded = sqlx::query!(
        "SELECT staff_onboarded FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(&data.pool)
    .await?;

    match onboard_state {
        "pending" | "queue-step" => {
            let test_bot = std::env::var("TEST_BOT")?;
            let bot_page = std::env::var("BOT_PAGE")?;
            
            if onboard_state == "pending" {
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

                return Ok(false)
            }

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
            } else {
                ctx.say("You can use the `/queue` command to see the list of bots pending verification that *you* need to review! Lets try that out?").await?;
            }

            Ok(false)
        },
        _ => Ok(true)
    }
}
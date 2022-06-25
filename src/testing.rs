use crate::checks;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::UserId;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Claims a bot
#[poise::command(
    prefix_command,
    slash_command,
    guild_cooldown = 10,
    check = "checks::is_staff"
)]
pub async fn claim(
    ctx: Context<'_>, 
    #[description = "The bot you wish to claim"]
    bot: serenity::Member
    ) -> Result<(), Error> {
    if !checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    // Check if its claimed by someone
    let data = ctx.data();
    let discord = ctx.discord();

    let claimed = sqlx::query!(
        "SELECT owner, claimed_by FROM bots WHERE bot_id = $1",
        bot.user.id.0.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    // Get main owner
    let owner = UserId(claimed.owner.parse::<u64>().unwrap());

    if claimed.claimed_by.is_none() || claimed.claimed_by.as_ref().unwrap().is_empty() {
        // Claim it
        sqlx::query!(
            "UPDATE bots SET claimed = true, claimed_by = $1 WHERE bot_id = $2",
            ctx.author().id.0.to_string(),
            bot.user.id.0.to_string()
        )
        .execute(&data.pool)
        .await?;
    } else {
        let mut msg = ctx.send(|m| {
            m.embed(|e| {
                e.title("Bot Already Claimed");
                e.description(format!("This bot is already claimed by <@{}>", claimed.claimed_by.as_ref().unwrap()));
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
        .message()
        .await?;

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
                        "<@{claimed_by}>, did you forgot to finish testing <@{bot_id}>?", 
                        claimed_by = claimed.claimed_by.unwrap(),
                        bot_id = bot.user.id.0
                    )
                ).await?;
            } else {
                // Force claim
                sqlx::query!(
                    "UPDATE bots SET claimed = true, claimed_by = $1 WHERE bot_id = $2",
                    ctx.author().id.0.to_string(),
                    bot.user.id.0.to_string()
                )
                .execute(&data.pool)
                .await?;

                let private_channel = owner.create_dm_channel(discord).await?;

                private_channel.send_message(discord, |m| {
                    m.embed(|e| {
                        e.title("Bot Reclaimed!");
                        e.description(format!("<@{}> has reclaimed <@{}> from <{}>", ctx.author().id.0, bot.user.id.0, claimed.claimed_by.unwrap()));
                        e.footer(|f| {
                            f.text("This is completely normal, don't worry!");
                            f
                        });
                        e
                    });
                    m
                })
                .await?;

                ctx.say(
                    format!(
                        "You have claimed <@{bot_id}> and the bot owner has been notified!", 
                        bot_id = bot.user.id.0
                    )
                ).await?;
            }
        } else {
            return Ok(())
        }

        return Ok(())
    }

    Ok(())
}

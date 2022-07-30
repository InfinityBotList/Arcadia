use crate::_checks as checks;

use poise::serenity_prelude::User;
use sqlx::Column;
use sqlx::Row;

use std::time::Duration;

use poise::serenity_prelude as serenity;

/// Allows managers to onboard users
#[poise::command(category = "Admin", track_edits, prefix_command, slash_command, check = "checks::is_hdev_hadmin")]
pub async fn approveonboard(
    ctx: crate::Context<'_>,
    #[description = "The staff id"] member: serenity::Member,
) -> Result<(), crate::Error> { 
    if !checks::staff_server(ctx).await? {
        return Err("You are not in the staff server".into());
    }

    let data = ctx.data();
    let discord = ctx.discord();

    // Check onboard state of user
    let onboard_state = sqlx::query!(
        "SELECT staff_onboard_state FROM users WHERE user_id = $1",
        member.user.id.to_string()
    )
    .fetch_one(&data.pool)
    .await?;

    if onboard_state.staff_onboard_state != "pending-manager-review" {
        return Err(format!("User is not pending manager review and currently has state of: {}", onboard_state.staff_onboard_state).into());
    }

    let mut msg = ctx.send(|m| {
        m.content("Are you sure you wish to approve this user?")
        .components(|c| {
            c.create_action_row(|r| {
                r.create_button(|b| {
                    b.custom_id("continue")
                    .label("Continue")
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
        .await;

    msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press

    let pressed_button_id = match &interaction {
        Some(m) => &m.data.custom_id,
        None => {
            ctx.say("You didn't interact in time").await?;
            return Ok(());
        }
    };

    if pressed_button_id == "cancel" {
        ctx.say("Cancelled").await?;
        return Ok(());
    }
    
    // Update onboard state of user
    sqlx::query!(
        "UPDATE users SET staff_onboard_state = 'complete' WHERE user_id = $1",
        member.user.id.to_string()
    )
    .execute(&data.pool)
    .await?;

    // DM user that they have been approved
    let _ = member.user.dm(&discord.http, |m| {
        m.content("Your onboarding request has been approved. You may now begin approving/denying bots")
    }).await?;

    ctx.say("Onboarding request approved!").await?;

    Ok(())
}

/// Returns a field on a specific bot id
#[poise::command(category = "Admin", track_edits, prefix_command, slash_command, check = "checks::is_hdev")]
pub async fn update_field(
    ctx: crate::Context<'_>,
    #[description = "The sql statement"] sql: String,
) -> Result<(), crate::Error> {
    if !checks::staff_server(ctx).await? {
        return Err("You are not in the staff server".into());
    }

    let data = ctx.data();

    if !sql.to_lowercase().contains(&"WHERE") {
        let mut msg = ctx
            .send(|m| {
                m.content("Whoa there, are you trying to update a whole table?.")
                    .components(|c| {
                        c.create_action_row(|r| {
                            r.create_button(|b| {
                                b.custom_id("yes")
                                    .style(serenity::ButtonStyle::Primary)
                                    .label("Yes")
                            });
                            r.create_button(|b| {
                                b.custom_id("cancel")
                                    .style(serenity::ButtonStyle::Secondary)
                                    .label("Cancel")
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
            .await;
        msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press

        if let Some(m) = &interaction {
            if m.data.custom_id != "yes" {
                return Err("Cancelled".into());
            }
        } else {
            return Ok(());
        }
    }

    // Ask for approval from someone else
    let mut msg = ctx
        .send(|m| {
            m.content(
                "Please have someone else approve running this SQL statement: ``".to_string()
                    + &sql
                    + "``",
            )
            .components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.custom_id("yes")
                            .style(serenity::ButtonStyle::Primary)
                            .label("Yes")
                    });
                    r.create_button(|b| {
                        b.custom_id("cancel")
                            .style(serenity::ButtonStyle::Secondary)
                            .label("Cancel")
                    })
                })
            })
        })
        .await?
        .into_message()
        .await?;

    // Get current iblhdev's

    let iblhdevs = sqlx::query!("SELECT user_id FROM users WHERE iblhdev = true")
        .fetch_all(&data.pool)
        .await?;

    let id = ctx.author().id;

    let interaction = msg
        .await_component_interaction(ctx.discord())
        .filter(move |f| {
            if f.user.id != id && iblhdevs.iter().any(|u| u.user_id == f.user.id.to_string()) {
                return true;
            }
            false
        })
        .timeout(Duration::from_secs(360))
        .await;
    msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press

    if let Some(m) = &interaction {
        if m.data.custom_id != "yes" {
            return Err("Cancelled".into());
        }
    } else {
        return Ok(());
    }

    let res = sqlx::query(&sql).fetch_all(&data.pool).await?;

    let mut sql_data = Vec::new();

    // Parse PgRow into a Vec<String>
    for row in res {
        let row = row;
        let mut row_data = Vec::new();
        for field in row.columns() {
            let field_str = format!("{:?}: {:?}", field.name(), serde_json::to_string(&field)?);
            row_data.push(field_str);
        }
        sql_data.push(row_data);
    }

    ctx.say("SQL statement executed").await?;

    // Split SQL into 1998 character chunks and keep sending
    let sql_full = format!("{:?}", sql_data);

    let mut sql_chunks = Vec::new();

    let mut sql_chunk = String::new();
    for (i, c) in sql_full.chars().enumerate() {
        sql_chunk.push(c);
        if i % 1998 == 0 && i > 0 {
            sql_chunks.push(sql_chunk.clone());
            sql_chunk.clear();
        }
    }

    for chunk in sql_chunks {
        if !chunk.is_empty() {
            ctx.say(chunk).await?;
        }
    }

    // Empty buffer
    if !sql_chunk.is_empty() {
        ctx.say(sql_chunk).await?;
    }

    Ok(())
}

#[poise::command(category = "Admin", track_edits, prefix_command, slash_command, check = "checks::is_hdev_hadmin")]
pub async fn votereset(
    ctx: crate::Context<'_>,
    #[description = "The bots ID"] bot: User,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    libavacado::manage::vote_reset(&ctx.discord(), &ctx.data().pool, &bot.id.to_string(), &ctx.author().id.to_string(), &reason).await
}

#[poise::command(category = "Admin", track_edits, prefix_command, slash_command, check = "checks::is_hdev_hadmin")]
pub async fn voteresetall(
    ctx: crate::Context<'_>,
    #[description = "The reason"] reason: String,
) -> Result<(), crate::Error> {
    libavacado::manage::vote_reset_all(&ctx.discord(), &ctx.data().pool, &ctx.author().id.to_string(), &reason).await
}
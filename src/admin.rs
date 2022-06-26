use crate::checks;

use sqlx::Row;
use sqlx::Column;

use std::time::Duration;

use poise::serenity_prelude as serenity;

/// Returns a field on a specific bot id
#[poise::command(track_edits, prefix_command, slash_command, check = "checks::is_hdev")]
pub async fn update_field(
    ctx: crate::Context<'_>,
    #[description = "The sql statement"] 
    sql: String,
) -> Result<(), crate::Error> {
    if !checks::staff_server(ctx).await? {
        return Err("You are not in the staff server".into());
    }

    let data = ctx.data();

    if !sql.to_lowercase().contains(&"WHERE") {
        let mut msg = ctx.send(|m| {
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
        .message()
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
            return Ok(())
        }
    }

    // Ask for approval from someone else
    let mut msg = ctx.send(|m| {
        m.content("Please have someone else approve running this SQL statement: ``".to_string() + &sql + "``")
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
    .message()
    .await?;

    // Get current iblhdev's

    let iblhdevs = sqlx::query!(
        "SELECT user_id FROM users WHERE iblhdev = true"
    )
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
        return Ok(())
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

    // Split SQL into 2000 character chunks and keep sending
    let sql_full = format!("{:?}", sql_data);

    let mut sql_chunks = Vec::new();

    let mut sql_chunk = String::new();
    for (i, c) in sql_full.chars().enumerate() {
        sql_chunk.push(c);
        if i % 2000 == 0 && i > 0 {
            sql_chunks.push(sql_chunk.clone());
            sql_chunk.clear();
        }
    }

    for chunk in sql_chunks {
        if !chunk.is_empty() {
            ctx.say(chunk).await?;
        }
    }

    Ok(())
}
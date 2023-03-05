// create table todo_list (id serial primary key, title text not null, description text not null, priority text not null, resolve_state text not null);

use std::time::Duration;

use futures_util::StreamExt;
use log::info;
use poise::{ChoiceParameter, serenity_prelude::{self as serenity, CreateEmbed, CreateSelectMenuOption, CreateActionRow, CreateButton, ComponentInteractionDataKind}, CreateReply};
use serde::Serialize;
use sqlx::PgPool;
use strum_macros::FromRepr;
use crate::checks;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

#[derive(FromRepr, ChoiceParameter, Serialize, Clone, Copy)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(FromRepr, ChoiceParameter, Serialize, Clone, Copy)]
pub enum ResolveState {
    Unresolved,
    Resolved,
    WontFix,
}

#[poise::command(
    slash_command,
    prefix_command,
    subcommands("todo_add", "todo_list")
)]
pub async fn todo(
    ctx: Context<'_>,
) -> Result<(), Error> {
    ctx.say("Available options are ``todo add``, ``todo resolve`` and ``todo list``").await?;

    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    rename = "add",
    check = "checks::is_admin_hdev",
)]
pub async fn todo_add(
    ctx: Context<'_>,
    #[description = "The title of the todo item"] title: String,
    #[description = "The description of the todo item"] description: String,
    #[description = "The priority of the todo item"] priority: Priority,
) -> Result<(), Error> {
    if description.len() > 4096 {
        return Err("The description is too long. Max=4096".into());
    }

    if title.len() > 100 {
        return Err("The title is too long. Max=100".into());
    }

    // Get number of todo items
    let count = sqlx::query!(
        "select count(*) from todo_list"
    )
    .fetch_one(&ctx.data().pool)
    .await?
    .count;

    if count.unwrap_or_default() > 2147483647 {
        return Err("There are too many todo items. Please remove some first.".into());
    }

    let todo = sqlx::query!(
        "insert into todo_list (title, description, priority, resolve_state) values ($1, $2, $3, $4) returning id",
        title,
        description,
        priority.to_string(),
        ResolveState::Unresolved.to_string()
    )
    .fetch_one(&ctx.data().pool)
    .await?;

    ctx.say(format!("Added todo item #{}", todo.id)).await?;

    Ok(())
}

/// Internal function that creates a select menu
fn _create_select_menu(titles: &[String], index: usize) -> serenity::builder::CreateSelectMenu {
    let mut options = Vec::new();

    for (i, pane) in titles.iter().enumerate() {
        if i+1 == index {
            options.push(CreateSelectMenuOption::new(
                pane.clone() + " (current)",
                (i+1).to_string(),
            ))
        } else {
            options.push(CreateSelectMenuOption::new(
                pane.clone(),
                (i+1).to_string(),
            ));
        }
    }

    serenity::builder::CreateSelectMenu::new(
        "todo:selectmenu",
        serenity::builder::CreateSelectMenuKind::String { options },
    )
    .custom_id("todo:selectmenu")
}

async fn _create_reply(
    id: i32,
    total_count: i32,
    titles: &[String],
    pool: &PgPool
) -> Result<CreateReply, Error> {
    if id < 1 {
        return Err("The ID is too low".into());
    }

    let entry_id = sqlx::query!(
        "select id from todo_list where id >= $1 limit 1",
        id
    )
    .fetch_one(pool)
    .await?;

    if id != entry_id.id {
        // Rename the ID on db as it is not the same sequentially
        sqlx::query!(
            "update todo_list set id = $1 where id = $2",
            id,
            entry_id.id
        )
        .execute(pool)
        .await?;        
    }

    let entry = sqlx::query!(
        "select id, title, description, priority, resolve_state from todo_list where id = $1",
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(
        CreateReply::default()
            .embed(
                CreateEmbed::default()
                    .title(format!("{} [{}/{}]", entry.title, id, total_count))
                    .description(&entry.description)
                    .field("Priority", entry.priority, true)
                    .field("Resolved", entry.resolve_state, true),
            )
            .components(vec![
                CreateActionRow::Buttons(vec![
                    CreateButton::new("todo:".to_string() + &(id - 1).to_string())
                        .label("Previous")
                        .disabled(id <= 1),
                    CreateButton::new("todo:cancel")
                        .label("Cancel")
                        .style(serenity::ButtonStyle::Danger),
                    CreateButton::new("todo:".to_string() + &(id + 1).to_string())
                        .label("Next")
                        .disabled(id == total_count),
                ]),
                CreateActionRow::SelectMenu(_create_select_menu(titles, id.try_into().unwrap_or_default())),
            ])
    )
}

#[poise::command(
    slash_command,
    prefix_command,
    rename = "list",
    check = "checks::is_admin_hdev",
)]
pub async fn todo_list(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let total_count = sqlx::query!(
        "select count(*)::integer from todo_list"
    )
    .fetch_one(&ctx.data().pool)
    .await?;

    // Get titles
    let titles_rec = sqlx::query!(
        "select title from todo_list order by id asc"
    )
    .fetch_all(&ctx.data().pool)
    .await?;

    let mut titles = Vec::new();

    for title in titles_rec {
        titles.push(title.title);
    }

    let total_count = total_count.count.unwrap_or_default();

    let entry = _create_reply(1, total_count, &titles, &ctx.data().pool).await?;

    // Send the message
    let msg = ctx.send(entry).await?.into_message().await?;

    let interaction = msg
    .await_component_interactions(ctx.discord())
    .author_id(ctx.author().id)
    .timeout(Duration::from_secs(120));

    let mut collect_stream = interaction.stream();

    while let Some(item) = collect_stream.next().await {
        item.defer(&ctx.discord()).await?;

        let id = &item.data.custom_id;

        info!("Received TODO interaction: {}", id);

        if id == "todo:cancel" {
            item.delete_response(ctx.discord()).await?;
            return Ok(());
        }

        if id == "todo:selectmenu" {
            // This is a select menu, get the value using modal_get
            let value = match item.data.kind {
                ComponentInteractionDataKind::StringSelect { ref values, .. } => {
                    if values.is_empty() {
                        return Err("Internal error: No value selected".into());
                    }

                    &values[0]
                }
                _ => {
                    return Err("Internal error: Invalid interaction type".into());
                }
            };

            let value = value.parse::<i32>()?;

            let entry = _create_reply(value, total_count, &titles, &ctx.data().pool).await?;

            // Edit message
            item.edit_response(&ctx, entry.to_slash_initial_response_edit()).await?;

            continue;
        }

        if id.starts_with("todo:") {
            let value = id.replace("todo:", "").parse::<i32>()?;

            let entry = _create_reply(value, total_count, &titles, &ctx.data().pool).await?;

            // Edit message
            item.edit_response(&ctx, entry.to_slash_initial_response_edit()).await?;
        }
    }

    Ok(())
}

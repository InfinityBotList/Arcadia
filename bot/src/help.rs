use std::fmt::Write;
use futures_util::StreamExt;
use poise::serenity_prelude::{self as serenity, ChannelId, MessageId};
use poise::Command;

use crate::Context;
use std::time::Duration;
use std::sync::Arc;
use crate::Error;
use crate::Data;
use log::info;

/// Struct to store embed data for the help command
struct EmbedHelp {
    category: String,
    desc: String,
}

async fn _embed_help(
    ctx: poise::FrameworkContext<'_, Data, Error>,
) -> Result<Vec<EmbedHelp>, Error> {
    let mut categories =
        libavacado::maps::OrderedMap::<Option<&str>, Vec<&Command<Data, Error>>>::new();
    for cmd in &ctx.options().commands {
        categories
            .get_or_insert_with(cmd.category, Vec::new)
            .push(cmd);
    }

    let mut help_arr = Vec::new();

    for (category_name, commands) in categories {
        let cat_name = category_name.unwrap_or("Commands");
        let mut menu = "".to_string();
        for command in commands {
            if command.hide_in_help {
                continue;
            }

            let _ = writeln!(
                menu,
                "/{cmd_name} | ibb!{cmd_name} - {desc}",
                cmd_name = command.name,
                desc = command.description.as_deref().unwrap_or("")
            );
        }

        help_arr.push(EmbedHelp {
            category: cat_name.to_string(),
            desc: menu.clone(),
        });
    }

    Ok(help_arr)
}

/// Instead of cloning a large Message struct, we use a temporary MsgInfo struct to store just the info we need
pub struct MsgInfo {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

async fn _help_send_index(ctx: Option<Context<'_>>, old_msg: Option<MsgInfo>, http: &Arc<serenity::Http>, data: &Vec<EmbedHelp>, index: usize) -> Result<Option<serenity::Message>, crate::Error> {
    let next_disabled = index >= data.len() - 1;

    let data = data.get(index);

    let prev_disabled = index == 0;

    match data {
        None => return Ok(None),
        Some(data) => {
            if let Some(old_msg) = old_msg {
                old_msg.channel_id.edit_message(http, old_msg.message_id, |m| {
                    m.embed(|e| {
                        e.title(format!("{} (Page {})", data.category, index + 1));
                        e.description(&data.desc);
                        e
                    })
                    .components(|c| {
                        c.create_action_row(|a| {
                            a.create_button(|b| {
                                b.label("Previous")
                                .custom_id("hnav:".to_string() + &(index - 1).to_string())
                                .disabled(prev_disabled)
                            })
                            .create_button(|b| {
                                b.label("Cancel")
                                .custom_id("hnav:cancel")
                                .style(serenity::ButtonStyle::Danger)
                            })
                            .create_button(|b| {
                                b.label("Next")
                                .custom_id("hnav:".to_string() + &(index + 1).to_string())
                                .disabled(next_disabled)
                            })
                        })
                    })
                })
                .await?;

                return Ok(None)
            }


            if let Some(ctx) = ctx {
                let msg = ctx.send(|m| {
                    m.embed(|e| {
                        e.title(format!("{} (Page {})", data.category, index + 1));
                        e.description(&data.desc);
                        e
                    })
                    .components(|c| {
                        c.create_action_row(|a| {
                            a.create_button(|b| {
                                b.label("Previous")
                                .custom_id("hnav:".to_string() + &(index - 1).to_string())
                                .disabled(prev_disabled)
                            })
                            .create_button(|b| {
                                b.label("Cancel")
                                .custom_id("hnav:cancel")
                                .style(serenity::ButtonStyle::Danger)
                            })
                            .create_button(|b| {
                                b.label("Next")
                                .custom_id("hnav:".to_string() + &(index + 1).to_string())
                                .disabled(next_disabled)
                            })
                        })
                    })
                })
                .await?
                .into_message()
                .await?;

                return Ok(Some(msg))
            }
        }
    }

    Ok(None)
}

#[poise::command(track_edits, prefix_command, slash_command)]
pub async fn new_help(ctx: Context<'_>) -> Result<(), Error> {
    let eh = _embed_help(ctx.framework()).await?;

    let msg = _help_send_index(Some(ctx), None, &ctx.discord().http, &eh, 0).await?;

    if let Some(msg) = msg {
        let mut interaction = msg
            .await_component_interactions(ctx.discord())
            .author_id(ctx.author().id)
            .timeout(Duration::from_secs(120))
            .build();
        
        while let Some(item) = interaction.next().await { 
            let id = &item.data.custom_id;

            info!("Received interaction: {}", id);

            if id == "hnav:cancel" {
                item.delete_original_interaction_response(ctx.discord()).await?;
                interaction.stop();
                break;
            }

            if id.starts_with("hnav:") {
                let id = id.replace("hnav:", "");
                let id = id.parse::<usize>()?;

                _help_send_index(None, Some(MsgInfo {
                    channel_id: msg.channel_id,
                    message_id: msg.id,
                }), &ctx.discord().http, &eh, id).await?;
            }
        }
    } else {
        return Err("No help message found".into())
    }

    Ok(())
}

#[poise::command(track_edits, prefix_command, slash_command)]
pub async fn maint(ctx: Context<'_>) -> Result<(), Error> {
    let maints = libavacado::public::maint_status()?;

    if maints.is_empty() {
        ctx.say("No maintenances are currently happening :)").await?;
        return Ok(());
    }

    ctx.send(|m| {
        for maint in maints {
            m.embed(|e| {
                e.title(maint.title);
                e.description(maint.description);
                e.color(0xFF0000);
                e
            });
        }
        m
    }).await?;

    Ok(())
}
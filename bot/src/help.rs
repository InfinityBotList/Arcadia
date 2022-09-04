use std::fmt::Write;
use futures_util::StreamExt;
use poise::serenity_prelude::{self as serenity, ChannelId, MessageId, MessageComponentInteraction};
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
                desc = command.description.as_deref().unwrap_or("*No description available yet*")
            ); 

            if !command.subcommands.is_empty() {

                let _ = writeln!(
                    menu,
                    "**Subcommands**",
                );

                for subcmd in command.subcommands.iter() {
                    if subcmd.hide_in_help {
                        continue;
                    }

                    let _ = writeln!(
                        menu,
                        "/{cmd_name} {subcmd_name} | ibb!{cmd_name} {subcmd_name} - {desc}",
                        cmd_name = command.name,
                        subcmd_name = subcmd.name,
                        desc = subcmd.description.as_deref().unwrap_or("*No description available yet*")
                    );
                }
            }
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

/// Internal function to populate the help action row (select menu)
#[inline]
fn _help_select_menu<'a, 'b>(data: &'b [EmbedHelp], ar: &'a mut serenity::builder::CreateActionRow, index: usize) -> &'a mut serenity::builder::CreateActionRow {            
    ar.create_select_menu(|sm| {
        sm.min_values(1)
        .max_values(1)
        .custom_id("hnav:selectmenu")
        .options(|opts| {
            for (i, pane) in data.iter().enumerate() {
                if i == index {
                    opts.create_option(|opt| {
                        opt.label(pane.category.clone() + " (current)")
                        .value(i.to_string())
                    });
                } else {
                    opts.create_option(|opt| {
                        opt.label(pane.category.clone())
                        .value(i.to_string())
                    });
                }
            }    

            opts
        }) 
    })
}


/// Internal function to populate the help action row (buttons)
#[inline]
fn _help_components(ar: &mut serenity::builder::CreateActionRow, index: usize, prev_disabled: bool, next_disabled: bool) -> &mut serenity::builder::CreateActionRow {            
    ar.create_button(|b| {
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
}

async fn _help_send_index(
    ctx: Option<Context<'_>>, 
    old_msg: Option<MsgInfo>, 
    http: &Arc<serenity::Http>, 
    l_data: &Vec<EmbedHelp>, 
    index: usize,
    interaction: Option<Arc<MessageComponentInteraction>>,
) -> Result<Option<serenity::Message>, crate::Error> {
    let next_disabled = index >= l_data.len() - 1;

    let data = l_data.get(index);

    let prev_disabled = index == 0;

    match data {
        None => return Ok(None),
        Some(data) => {
            if let Some(old_msg) = old_msg {
                if interaction.is_none() {
                    old_msg.channel_id.edit_message(http, old_msg.message_id, |m| {
                        m.embed(|e| {
                            e.title(format!("{} (Page {})", data.category, index + 1));
                            e.description(&data.desc);
                            e
                        })
                        .components(|c| {
                            c.create_action_row(|a| {
                                _help_components(a, index, prev_disabled, next_disabled)
                            })
                            .create_action_row(|ar| {
                                _help_select_menu(l_data, ar, index)
                            })
                        })
                    })
                    .await?;
                } else {
                    let interaction = interaction.unwrap();

                    interaction.edit_original_interaction_response(http, |m| {
                        m.embed(|e| {
                            e.title(format!("{} (Page {})", data.category, index + 1));
                            e.description(&data.desc);
                            e
                        })
                        .components(|c| {
                            c.create_action_row(|a| {
                                _help_components(a, index, prev_disabled, next_disabled)
                            })
                            .create_action_row(|ar| {
                                _help_select_menu(l_data, ar, index)
                            })
                        })
                    }).await?;
                }

                return Ok(None)
            }

            if let Some(ctx) = ctx {
                let msg = ctx.send(|m| {
                    m.ephemeral(true);

                    m.embed(|e| {
                        e.title(format!("{} (Page {})", data.category, index + 1));
                        e.description(&data.desc);
                        e
                    })
                    .components(|c| {
                        c.create_action_row(|a| {
                            _help_components(a, index, prev_disabled, next_disabled)
                        })
                        .create_action_row(|ar| {
                            _help_select_menu(l_data, ar, index)
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

    let msg = _help_send_index(Some(ctx), None, &ctx.discord().http, &eh, 0, None).await?;

    if let Some(msg) = msg {
        let mut interaction = msg
            .await_component_interactions(ctx.discord())
            .author_id(ctx.author().id)
            .timeout(Duration::from_secs(120))
            .build();
        
        while let Some(item) = interaction.next().await { 
            item.defer(&ctx.discord()).await?;

            let id = &item.data.custom_id;

            info!("Received interaction: {}", id);

            if id == "hnav:cancel" {
                item.delete_original_interaction_response(ctx.discord()).await?;
                interaction.stop();
                break;
            }

            if id == "hnav:selectmenu" {
                // This is a select menu, get the value using modal_get
                let value = crate::_utils::ModalValue::new(item.data.values.clone());

                let value = value.extract_single();

                if value.is_none() {
                    continue;
                }

                let value = value.unwrap().parse::<usize>()?;

                _help_send_index(
                    None, 
                    Some(
                        MsgInfo {
                            channel_id: msg.channel_id,
                            message_id: msg.id,
                        }
                    ),
                    &ctx.discord().http, 
                    &eh, 
                    value,
                    Some(item.clone()),
                ).await?;

                continue;
            }

            if id.starts_with("hnav:") {
                let id = id.replace("hnav:", "");
                let id = id.parse::<usize>()?;

                _help_send_index(
                    None, 
                    Some(
                        MsgInfo {
                            channel_id: msg.channel_id,
                            message_id: msg.id,
                        }
                    ),
                    &ctx.discord().http, 
                    &eh, 
                    id,
                    Some(item.clone()),
                ).await?;
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
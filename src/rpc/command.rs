use std::str::FromStr;
use std::time::Duration;

use poise::serenity_prelude::{InteractionId, CreateActionRow, CreateButton, ButtonStyle, CreateQuickModal, CreateInputText, InputTextStyle};
use poise::CreateReply;
use strum::VariantNames;

use crate::{Context, Error};

async fn autocomplete(
    _ctx: Context<'_>,
    partial: &str
) -> Vec<poise::AutocompleteChoice<String>> {
    let mut choices = Vec::new();

    for m in super::core::RPCMethod::VARIANTS {
        if partial.is_empty() || m.contains(partial) {
            choices.push(poise::AutocompleteChoice {
                name: m.to_string(),
                value: m.to_string(),
            });
        }
    }

    choices
}

fn parse_bool(v: &str) -> Result<bool, Error> {
    match v.to_lowercase().as_str() {
        "true" | "t" => Ok(true),
        "false" | "f" => Ok(false),
        _ => Err("Invalid boolean".into()),
    }
}

fn parse_hrs(v: &str) -> Result<i32, Error> {
    // Split v into time and unit
    let data = v.split(' ').collect::<Vec<&str>>();

    if data.len() != 2 {
        return Err("Invalid time format. Format must be WITH A SPACE BETWEEN THE NUMBER AND THE UNIT".into());
    }

    let (time, unit) = (data[0], data[1]);

    let time = time.parse::<i32>()?;

    match unit {
        "years" | "year" | "y" => Ok(time * 365 * 24),
        "months" | "month" | "mo" | "m" => Ok(time * 30 * 24),
        "weeks" | "week" | "w" => Ok(time * 7 * 24),
        "days" | "day" | "d" => Ok(time * 24),
        "hours" | "hour" | "hrs" | "hr" | "h" => Ok(time),
        _ => Err("Invalid time format. Unit must be years, months, weeks, days or hours".into()),
    }
}

/// Converts a variant to a proper method using a modal
async fn get_method(
    ctx: &poise::serenity_prelude::Context,
    variant: super::core::RPCMethod,
    inter_id: InteractionId,
    inter_token: &str,
) -> Result<super::core::RPCMethod, Error> {
    match variant {
        super::core::RPCMethod::BotApprove { .. } => {
            let qm = CreateQuickModal::new("Approve Bot")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason) = (&inputs[0], &inputs[1]);
                
                Ok(super::core::RPCMethod::BotApprove {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotDeny { .. } => {
            let qm = CreateQuickModal::new("Deny Bot")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason) = (&inputs[0], &inputs[1]);
                
                Ok(super::core::RPCMethod::BotDeny {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotVoteReset { .. } => {
            let qm = CreateQuickModal::new("Vote Reset Bot")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason) = (&inputs[0], &inputs[1]);
                
                Ok(super::core::RPCMethod::BotVoteReset {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotVoteResetAll { .. } => {
            let qm = CreateQuickModal::new("Vote Reset All Bots")
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let reason = &inputs[0];
                
                Ok(super::core::RPCMethod::BotVoteResetAll {
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotUnverify { .. } => {
            let qm = CreateQuickModal::new("Unverify Bot")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason").placeholder("You must give proof"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason) = (&inputs[0], &inputs[1]);
                
                Ok(super::core::RPCMethod::BotUnverify {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotPremiumAdd { .. } => {
            let qm = CreateQuickModal::new("Add Bot To Premium")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason").placeholder("You must give proof"))
            .field(CreateInputText::new(InputTextStyle::Short, "Time Period", "time_period").placeholder("Format: INTEGER UNIT, e.g. 1 day, 2 weeks, 3 months, 4 years"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason, time_period_str) = (&inputs[0], &inputs[1], &inputs[2]);
                
                Ok(super::core::RPCMethod::BotPremiumAdd {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                    time_period_hours: parse_hrs(time_period_str)?,
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotPremiumRemove { .. } => {
            let qm = CreateQuickModal::new("Remove Bot From Premium")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason").placeholder("You must give proof"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason) = (&inputs[0], &inputs[1]);
                
                Ok(super::core::RPCMethod::BotPremiumRemove {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotVoteBanAdd { .. } => {
            let qm = CreateQuickModal::new("Vote Ban Bot")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason) = (&inputs[0], &inputs[1]);
                
                Ok(super::core::RPCMethod::BotVoteBanAdd {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotVoteBanRemove { .. } => {
            let qm = CreateQuickModal::new("Vote Ban Remove Bot")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason) = (&inputs[0], &inputs[1]);
                
                Ok(super::core::RPCMethod::BotVoteBanRemove {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotForceRemove { .. } => {
            let qm = CreateQuickModal::new("Force Remove Bot")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"))
            .field(CreateInputText::new(InputTextStyle::Short, "Kick?", "kick").placeholder("T/F"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason, kick) = (&inputs[0], &inputs[1], &inputs[2]);
                
                Ok(super::core::RPCMethod::BotForceRemove {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                    kick: parse_bool(kick)?,
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotCertifyRemove { .. } => {
            let qm = CreateQuickModal::new("Uncertify Bot")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason) = (&inputs[0], &inputs[1]);
                
                Ok(super::core::RPCMethod::BotCertifyRemove {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                })
            } else {
                Err("No response".into())
            }
        },
        super::core::RPCMethod::BotVoteCountSet { .. } => {
            let qm = CreateQuickModal::new("Set Bot Vote Count")
            .field(CreateInputText::new(InputTextStyle::Short, "Bot ID", "bot_id"))
            .field(CreateInputText::new(InputTextStyle::Paragraph, "Reason", "reason"))
            .field(CreateInputText::new(InputTextStyle::Short, "Count", "count"));

            if let Some(resp) = qm.execute(ctx, inter_id, inter_token).await? {
                let inputs = resp.inputs;
                let (bot_id, reason, count_str) = (&inputs[0], &inputs[1], &inputs[2]);
                
                Ok(super::core::RPCMethod::BotVoteCountSet {
                    bot_id: bot_id.to_string(),
                    reason: reason.to_string(),
                    count: count_str.parse::<i32>()?,
                })
            } else {
                Err("No response".into())
            }
        },
    }
}

#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    broadcast_typing,
    check = "crate::checks::is_staff",
)]
pub async fn rpc(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete"]
    method: String
) -> Result<(), Error> {
    // Creates a "blank" RPCMethod
    let variant = super::core::RPCMethod::from_str(&method)?;

    let rpc_method = match ctx {
        poise::Context::Application(app) => {
            match app.interaction {
                poise::CommandOrAutocompleteInteraction::Command(cmd) => get_method(ctx.discord(), variant, cmd.id, &cmd.token).await,
                poise::CommandOrAutocompleteInteraction::Autocomplete(_) => Err("Autocomplete cannot be used as a command argument".into()),
            }
        },
        poise::Context::Prefix(_) => {
            // Send modal button
            let builder = CreateReply::default()
            .content("OK, we just need some extra information first, please click the below button to launch a modal asking for more information")
            .components(
                vec![
                    CreateActionRow::Buttons(
                        vec![
                            CreateButton::new("next")
                            .label("Next")
                            .style(ButtonStyle::Primary),
                            CreateButton::new("cancel")
                            .label("Cancel")
                            .style(ButtonStyle::Danger)
                        ]
                    )
                ]
            );

            let mut msg = ctx.send(builder.clone()).await?.into_message().await?;

            let interaction = msg
                .await_component_interaction(ctx.discord())
                .author_id(ctx.author().id)
                .timeout(Duration::from_secs(120))
                .await;

            if let Some(m) = &interaction {
                let id = &m.data.custom_id;

                msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![]))
                    .await?; // remove buttons after button press

                if id == "next" {
                    get_method(ctx.discord(), variant, m.id, &m.token).await
                } else {
                    return Ok(())
                }
            } else {
                msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![]))
                    .await?; // remove buttons after timeout
                return Ok(())
            }
        },
    }?;

    if rpc_method.to_string() != method {
        return Err(
            format!(
                "Internal error: method ({}) != variant ({})",
                rpc_method,
                method
            ).into()
        )
    }

    let data = ctx.data();

    match rpc_method.handle(
        crate::rpc::core::RPCHandle {
            cache_http: data.cache_http.clone(),
            pool: data.pool.clone(),
            user_id: ctx.author().id.to_string(),
        }
    ).await? {
        super::core::RPCSuccess::NoContent => {
            ctx.say(
                format!(
                    "Successfully performed the operation required: `{}`",
                    rpc_method
                )
            ).await?;
            Ok(())
        }
        super::core::RPCSuccess::Content(msg) => {
            ctx.say(
                format!(
                    "Successfully performed the operation required: `{}`\n**{}**",
                    rpc_method,
                    msg
                )
            ).await?;
            Ok(())
        }
    }
}
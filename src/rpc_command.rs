use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use poise::serenity_prelude::{
    ButtonStyle, CreateActionRow, CreateButton, CreateInputText, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateQuickModal, InputTextStyle, ModalInteraction,
};
use poise::CreateReply;
use strum::VariantNames;

use crate::impls::target_types::TargetType;
use crate::rpc::core::RPCMethod;
use crate::{Context, Error};

async fn autocomplete(_ctx: Context<'_>, partial: &str) -> Vec<poise::AutocompleteChoice<String>> {
    let mut choices = Vec::new();

    for m in crate::rpc::core::RPCMethod::VARIANTS {
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
        "true" | "t" | "y" => Ok(true),
        "false" | "f" | "n" => Ok(false),
        _ => Err("Invalid boolean".into()),
    }
}

fn parse_hrs(v: &str) -> Result<i32, Error> {
    // Split v into time and unit
    let data = v.split(' ').collect::<Vec<&str>>();

    if data.len() != 2 {
        return Err(
            "Invalid time format. Format must be WITH A SPACE BETWEEN THE NUMBER AND THE UNIT"
                .into(),
        );
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

struct GetResp {
    method: crate::rpc::core::RPCMethod,
    interaction: ModalInteraction,
}

#[derive(poise::ChoiceParameter)]
pub enum TargetTypeChoice {
    Bot,
    Server,
    Team,
    Pack,
}

impl From<TargetTypeChoice> for TargetType {
    fn from(choice: TargetTypeChoice) -> Self {
        match choice {
            TargetTypeChoice::Bot => TargetType::Bot,
            TargetTypeChoice::Server => TargetType::Server,
            TargetTypeChoice::Team => TargetType::Team,
            TargetTypeChoice::Pack => TargetType::Pack,
        }
    }
}

#[poise::command(
    prefix_command,
    slash_command,
    check = "crate::checks::is_staff"
)]
pub async fn rpc(
    ctx: Context<'_>,
    target_type: TargetTypeChoice,
    #[autocomplete = "autocomplete"] method: String,
) -> Result<(), Error> {
    // Creates a "blank" RPCMethod
    let variant = crate::rpc::core::RPCMethod::from_str(&method)?;
    let discord = ctx.discord();

    let rpc_method = {
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

            if id == "cancel" {
                return Ok(());
            }

            let method_fields = variant.method_fields();

            let qm = {
                let mut qm = CreateQuickModal::new(variant.label());
            
                for field in method_fields.iter() {
                    qm = qm.field(CreateInputText::new(
                        match field.field_type {
                            crate::rpc::core::FieldType::Text => InputTextStyle::Short,
                            crate::rpc::core::FieldType::Textarea => InputTextStyle::Paragraph,
                            _ => InputTextStyle::Short
                        },
                        field.label.clone(),
                        field.id.clone(),
                    ).placeholder(field.placeholder.clone()));
                }
            
                qm
            };
                        
            if let Some(resp) = m.quick_modal(discord, qm).await? { 
                let mut data = HashMap::new();

                for (i, inp) in resp.inputs.iter().enumerate() {
                    if let Some(field) = &method_fields.get(i) {
                        let id = &field.id;
                        data.insert(id.clone(), inp.clone());
                    } else {
                        return Err("Internal error: field not found".into());
                    };
                }

                let method: RPCMethod = serde_json::from_value(serde_json::json!({
                    method: data
                }))?;

                GetResp {
                    method,
                    interaction: resp.interaction,
                }
            } else {
                return Err("Timed out waiting for modal response".into());
            }
        } else {
            msg.edit(ctx.discord(), builder.to_prefix_edit().components(vec![]))
                .await?; // remove buttons after timeout
            return Ok(());
        }
    };

    let data = ctx.data();

    match rpc_method
        .method
        .handle(crate::rpc::core::RPCHandle {
            cache_http: data.cache_http.clone(),
            pool: data.pool.clone(),
            user_id: ctx.author().id.to_string(),
            target_type: target_type.into(),
        })
        .await
    {
        Ok(resp) => match resp {
            crate::rpc::core::RPCSuccess::NoContent => {
                rpc_method
                    .interaction
                    .create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default().content(format!(
                                "Successfully performed the operation required: `{}`",
                                rpc_method.method
                            )),
                        ),
                    )
                    .await?;

                Ok(())
            }
            crate::rpc::core::RPCSuccess::Content(msg) => {
                rpc_method
                    .interaction
                    .create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default().content(format!(
                                "Successfully performed the operation required: `{}`\n**{}**",
                                rpc_method.method, msg
                            )),
                        ),
                    )
                    .await?;
                Ok(())
            }
        },
        Err(e) => {
            rpc_method
                .interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default().content(format!(
                            "Error performing `{}`: **{}**",
                            rpc_method.method, e
                        )),
                    ),
                )
                .await?;
            Ok(())
        }
    }
}

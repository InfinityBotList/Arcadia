use crate::{Context, Error};
use poise::{serenity_prelude::{CreateQuickModal, CreateInputText, InputTextStyle}, CommandOrAutocompleteInteraction};
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};

/// Unlocks RPC for a 10 minutes, is logged
#[poise::command(
    slash_command
)]
pub async fn modaltest(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let qm = CreateQuickModal::new("Test")
    .field(CreateInputText::new(
        InputTextStyle::Short,
        "HI",
        "h",
    ));

    match ctx {
        poise::structs::Context::Application(a) =>  {
            match a.interaction {
                CommandOrAutocompleteInteraction::Command(c) => {
                    if let Some(resp) = c.quick_modal(&ctx.discord(), qm).await? {
                        let inputs = resp.inputs;
                        let h = &inputs[0];

                        c.create_response(
                            ctx,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::default().content(h),
                            ),
                        )
                        .await?;        
                    }                
                },
                _ => {
                    return Err("Not a command".into());
                }
            }
        },
        _ => {
            return Err("Not an application context".into());
        }
    }

    Ok(())
}
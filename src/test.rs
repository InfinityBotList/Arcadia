use crate::{Context, Error};
use poise::serenity_prelude::{CreateInputText, CreateQuickModal, InputTextStyle};
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};

/// Unlocks RPC for a 10 minutes, is logged
#[poise::command(slash_command)]
pub async fn modaltest(ctx: Context<'_>) -> Result<(), Error> {
    let qm =
        CreateQuickModal::new("Test").field(CreateInputText::new(InputTextStyle::Short, "HI", "h"));

    match ctx {
        poise::structs::Context::Application(a) => {
            if let Some(resp) = a
                .interaction
                .quick_modal(ctx.serenity_context(), qm)
                .await?
            {
                let inputs = resp.inputs;
                let h = &inputs[0];

                a.interaction
                    .create_response(
                        &ctx.serenity_context().http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default().content(h.to_string()),
                        ),
                    )
                    .await?;
            }
        }
        _ => {
            return Err("Not an application context".into());
        }
    }

    Ok(())
}

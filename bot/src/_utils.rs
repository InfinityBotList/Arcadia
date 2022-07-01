use log::error;
use poise::serenity_prelude::{self as serenity, ActionRowComponent};
use rand::{distributions::Alphanumeric, Rng};

pub fn gen_random(length: usize) -> String {
    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();

    s
}

pub async fn delete_leave_guild(
    http: &serenity::http::Http,
    cache: &serenity::Cache,
    guild_id: serenity::GuildId,
) {
    let gowner = cache.guild_field(guild_id, |g| g.owner_id).unwrap();

    if gowner == cache.current_user_id() {
        let err = guild_id.delete(http).await;

        if err.is_err() {
            error!(
                "Error while deleting guild with ID: {:?} (error: {:?})",
                guild_id,
                err.unwrap_err()
            );
        }
    } else {
        let err = guild_id.leave(http).await;

        if err.is_err() {
            error!(
                "Error while leaving guild with ID: {:?} (error: {:?})",
                guild_id,
                err.unwrap_err()
            );
        }
    }
}

/// For future use
pub async fn _page_content(
    ctx: crate::Context<'_>,
    text: String,
    ephemeral: bool,
) -> Result<Vec<poise::ReplyHandle>, crate::Error> {
    let mut text_chunks = Vec::new();

    let mut text_chunk = String::new();
    for (i, c) in text.chars().enumerate() {
        text_chunk.push(c);
        if i % 2000 == 0 && i > 0 {
            text_chunks.push(text_chunk.clone());
            text_chunk.clear();
        }
    }

    let mut chunks = Vec::new();

    for chunk in text_chunks {
        chunks.push(ctx.send(|m| m.content(chunk).ephemeral(ephemeral)).await?);
    }

    // Empty buffer
    if !text_chunk.is_empty() {
        chunks.push(ctx.send(|m| m.content(text_chunk).ephemeral(ephemeral)).await?);
    }

    Ok(chunks)
}

/// Get the action row component given id
/// In buttons, this returns 'found' if found in response
pub fn modal_get(resp: &serenity::ModalSubmitInteractionData, id: &str) -> String {
    for row in &resp.components {
        for component in &row.components {
            let id = id.to_string();

            match component {
                ActionRowComponent::Button(c) => {
                    if c.custom_id == Some(id) {
                        return "found".to_string()
                    }
                }
                ActionRowComponent::SelectMenu(s) => {
                    if s.custom_id == Some(id) {
                        todo!()
                    }
                }
                ActionRowComponent::InputText(t) => {
                    if t.custom_id == id {
                        return t.value.clone()
                    }
                }
                _ => {}
            }
        }
    }

    String::new()
}
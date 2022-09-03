use log::error;
use poise::serenity_prelude::{self as serenity, ActionRowComponent};

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
pub async fn page_content(
    ctx: crate::Context<'_>,
    text: String,
    ephemeral: bool,
) -> Result<Vec<poise::ReplyHandle>, crate::Error> {
    let mut text_chunks = Vec::new();

    let mut text_chunk = String::new();
    for (i, c) in text.chars().enumerate() {
        text_chunk.push(c);
        if i % 1998 == 0 && i > 0 {
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
        chunks.push(
            ctx.send(|m| m.content(text_chunk).ephemeral(ephemeral))
                .await?,
        );
    }

    Ok(chunks)
}

/// A Modal value struct
pub struct ModalValue {
    pub values: Option<Vec<String>>,
}

impl ModalValue {
    /// Returns the value from a Option<Vec<String>> returned by modal_get
    pub fn extract_single(self) -> Option<String> {
        self.values.as_ref()?;

        let resp = self.values.unwrap();

        if resp.is_empty() {
            return None;
        }

        let resp = &resp[0];

        if resp.is_empty() {
            return None;
        }

        Some(resp.to_string())
    }
}

/// Get the action row component given id
/// In buttons, this returns 'found' if found in response
/// In a select menu, values are returned as a string
pub fn modal_get(resp: &serenity::ModalSubmitInteractionData, id: &str) -> ModalValue {
    for row in &resp.components {
        for component in &row.components {
            let id = id.to_string();

            match component {
                ActionRowComponent::Button(c) => {
                    if c.custom_id == Some(id) {
                        return ModalValue { values: Some(vec!["found".to_string()]) };
                    }
                }
                ActionRowComponent::SelectMenu(s) => {
                    if s.custom_id == Some(id) {
                        return ModalValue { values: Some(s.values.clone()) };
                    }
                }
                ActionRowComponent::InputText(t) => {
                    if t.custom_id == id {
                        return ModalValue { values: Some(vec![t.value.clone()]) };
                    }
                }
                _ => {}
            }
        }
    }

    ModalValue { values: None }
}

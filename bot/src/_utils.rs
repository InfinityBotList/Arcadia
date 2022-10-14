use std::time::Duration;

use futures_util::StreamExt;
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

/// A Modal value struct (for handling select menus as well)
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

    // ``new`` on ModalValue
    pub fn new(values: Vec<String>) -> Self {
        Self { values: Some(values) }
    }
}


/// Get the action row component given id (for modals)
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

/// Poise doesn't seem to handle this anymore
#[derive(poise::ChoiceParameter)]
pub enum Bool {
    #[name = "True"]
    True,
    #[name = "False"]
    False,
}

impl Bool {
    pub fn to_bool(&self) -> bool {
        match self {
            Bool::True => true,
            Bool::False => false,
        }
    }
}

#[derive(Default, Debug)]
pub struct VoteData {
    pub approving_users: Vec<serenity::UserId>,
    pub disapproving_users: Vec<serenity::UserId>,
    pub cancelled: bool,
    pub winning_side: Option<bool>,
    pub forced: bool
}

impl VoteData {
    pub fn display(&self) -> String {
        let mut text = String::new();

        if !self.approving_users.is_empty() {
        text.push_str("**Approving users**\n");
            for user in &self.approving_users {
                text.push_str(&format!("{user_id}\n (<@{user_id}>)", user_id=user));
            }

            text.push_str("\n\n");
        }

        if !self.disapproving_users.is_empty() {
            text.push_str("\n\n**Disapproving users**\n");
            for user in &self.disapproving_users {
                text.push_str(&format!("{user_id}\n (<@{user_id}>)", user_id=user));
            }
        }

        text
    }

    pub fn total_voters(&self) -> usize {
        self.approving_users.len() + self.disapproving_users.len()
    }

    pub fn get_winning_side(&self, total_elgibile: usize) -> Option<bool> {
        let total_approving = self.approving_users.len();
        let total_disapproving = self.disapproving_users.len();

        let total_voters = total_approving + total_disapproving;

        // Firstly, if total_voters who voted is less than 50% of total_elgibile, return None
        if total_voters * 2 < total_elgibile {
            return None;
        }

        if total_approving > total_disapproving {
            return Some(true);
        }

        if total_disapproving > total_approving {
            return Some(false);
        }

        None
    }
}

pub async fn create_vote(
    ctx: crate::Context<'_>,
    vote_title: &str,
    can_vote: Vec<serenity::UserId>,
) -> Result<VoteData, crate::Error> {
    let mut vote_data = VoteData::default();

    let mut msg = ctx
        .send(|m| {
            m.content("**".to_string()+vote_title+"**\n\nThis message will timeout in 15 minutes" + "\n\n" + &vote_data.display())
                .ephemeral(true)
                .components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.style(serenity::ButtonStyle::Primary)
                                .label("Approve")
                                .custom_id("approve")
                        })
                        .create_button(|b| {
                            b.style(serenity::ButtonStyle::Primary)
                                .label("Disapprove")
                                .custom_id("disapprove")
                        })
                    })
                    .create_action_row(|r| {
                        r.create_button(|b| {
                            b.style(serenity::ButtonStyle::Danger)
                                .label("Cancel")
                                .custom_id("cancel")
                        })
                        .create_button(|b| {
                            b.style(serenity::ButtonStyle::Secondary)
                                .label("Force Resolve")
                                .custom_id("force_resolve")
                        })
                        .create_button(|b| {
                            b.style(serenity::ButtonStyle::Danger)
                                .label("Force Poll Through")
                                .custom_id("force_poll_through")
                        })
                    })
                })
        })
        .await?
        .into_message()
        .await?;

    let mut interaction = msg
        .await_component_interactions(ctx.discord())
        .timeout(Duration::from_secs(60 * 15))
        .build();
    
    while let Some(item) = interaction.next().await {
        if !can_vote.contains(&item.user.id) {
            item.create_interaction_response(&ctx.discord(), |r| {
                r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|d| {
                        d.content("You are not allowed to vote on this poll")
                        .ephemeral(true)
                    })
            })
            .await?;
            continue;
        }

        let id = item.data.custom_id.as_str();

        match id {
            "approve" => {
                if vote_data.disapproving_users.contains(&item.user.id) {
                    vote_data.disapproving_users.retain(|x| x != &item.user.id);
                }

                if vote_data.approving_users.contains(&item.user.id) {
                    item.create_interaction_response(&ctx.discord(), |r| {
                        r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|d| {
                                d.content("You have already approved this poll")
                                .ephemeral(true)
                            })
                    })
                    .await?;
                    continue;
                } else {
                    item.create_interaction_response(&ctx.discord(), |r| {
                        r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|d| {
                                d.content("You have made your vote")
                                .ephemeral(true)
                            })
                    })
                    .await?;
                }

                vote_data.approving_users.push(item.user.id);

                if vote_data.total_voters() >= can_vote.len() {
                    vote_data.winning_side = vote_data.get_winning_side(can_vote.len());
                    msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press
                    return Ok(vote_data);
                }
            }
            "disapprove" => {
                if vote_data.approving_users.contains(&item.user.id) {
                    vote_data.approving_users.retain(|x| x != &item.user.id);
                }

                if vote_data.disapproving_users.contains(&item.user.id) {
                    item.create_interaction_response(&ctx.discord(), |r| {
                        r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|d| {
                                d.content("You have already disapproved this poll")
                                .ephemeral(true)
                            })
                    })
                    .await?;
                    continue;
                } else {
                    item.create_interaction_response(&ctx.discord(), |r| {
                        r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|d| {
                                d.content("You have made your vote")
                                .ephemeral(true)
                            })
                    })
                    .await?;
                }

                vote_data.disapproving_users.push(item.user.id);

                if vote_data.total_voters() >= can_vote.len() {
                    vote_data.winning_side = vote_data.get_winning_side(can_vote.len());
                    msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press
                    return Ok(vote_data);
                }
            }
            "cancel" => {
                item.create_interaction_response(&ctx.discord(), |r| {
                    r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|d| {
                            d.content("Vote cancelled")
                        })
                })
                .await?;

                vote_data.cancelled = true;

                vote_data.winning_side = vote_data.get_winning_side(can_vote.len());

                msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press
                return Ok(vote_data);
            },
            "force_resolve" => {
                item.create_interaction_response(&ctx.discord(), |r| {
                    r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|d| {
                            d.content("Poll resolved")
                            .ephemeral(true)
                        })
                })
                .await?;

                // Get the winning side
                vote_data.winning_side = vote_data.get_winning_side(can_vote.len());

                msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press
                return Ok(vote_data);
            },
            "force_poll_through" => {
                item.create_interaction_response(&ctx.discord(), |r| {
                    r.kind(serenity::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|d| {
                            d.content("Vote forced through")
                            .ephemeral(true)
                        })
                })
                .await?;

                vote_data.forced = true;
                vote_data.approving_users = can_vote.clone(); // Force the poll through

                msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press
                return Ok(vote_data)
            },
            _ => {}
        }

        // Update the message
        msg.edit(&ctx.discord(), |m| {
            m.content("**".to_string()+vote_title+"**\n\nThis message will timeout in 15 minutes" + "\n\n" + &vote_data.display())
        })
        .await?;
    }
    
    // Get the winning side
    vote_data.winning_side = vote_data.get_winning_side(can_vote.len());

    msg.edit(ctx.discord(), |b| b.components(|b| b)).await?; // remove buttons after button press
    
    interaction.stop();

    Ok(vote_data)
}
use poise::CreateReply;

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
        let msg = CreateReply::default().content(chunk).ephemeral(ephemeral);
        chunks.push(ctx.send(msg).await?);
    }

    // Empty buffer
    if !text_chunk.is_empty() {
        let msg = CreateReply::default().content(text_chunk).ephemeral(ephemeral);
        chunks.push(ctx.send(msg).await?);
    }

    Ok(chunks)
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

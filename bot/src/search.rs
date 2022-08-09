type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

#[poise::command(category = "Search", prefix_command, slash_command, user_cooldown = 1)]
pub async fn searchbots(
    ctx: Context<'_>,
    #[description = "Search Query"] query: String,
) -> Result<(), Error> {
    let data = ctx.data();

    let search_res =
        libavacado::search::search_bots(&query, &data.pool, &data.avacado_public).await?;

    let mut msg = "**Bots**\n".to_string();

    for bot in &search_res.bots {
        msg.push_str(&(docser::serialize_docs(bot)?));
    }

    msg += "**Packs**\n";

    for pack in &search_res.packs {
        msg.push_str(&(docser::serialize_docs(pack)?));
    }

    msg += "**Users**\n";

    for user in &search_res.users {
        msg.push_str(&(docser::serialize_docs(user)?));
    }

    crate::_utils::page_content(ctx, msg, false).await?;

    Ok(())
}

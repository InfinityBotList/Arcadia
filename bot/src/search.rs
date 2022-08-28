use libavacado::search::{SearchOpts, SearchFilter};

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

#[poise::command(category = "Search", prefix_command, slash_command, user_cooldown = 1)]
pub async fn searchbots(
    ctx: Context<'_>,
    #[description = "Search Query"] query: String,
    #[description = "Search Count (FROM)"] gc_from: Option<i32>,
    #[description = "Search Count (TO)"] gc_to: Option<i32>,
    #[description = "Votes Count (FROM)"] votes_from: Option<i32>,
    #[description = "Votes Count (TO)"] votes_to: Option<i32>,
) -> Result<(), Error> {
    let data = ctx.data();

    let search_res =
        libavacado::search::search_bots(&query, &data.pool, &data.avacado_public, &SearchOpts {
            gc: SearchFilter {
                from: gc_from,
                to: gc_to,
            },
            votes: SearchFilter {
                from: votes_from,
                to: votes_to,
            }, 
        }).await?;

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

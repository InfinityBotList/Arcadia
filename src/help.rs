use crate::Context;
use crate::Error;

#[poise::command(category = "Help", track_edits, prefix_command, slash_command)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result<(), Error> {
    let prefix = crate::config::CONFIG.prefix.get();
    let prefix_str = prefix.as_str();

    botox::help::help(
        ctx,
        command,
        prefix_str,
        botox::help::HelpOptions::default(),
    )
    .await
}

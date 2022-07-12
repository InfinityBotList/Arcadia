type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

#[poise::command(category = "Stats", prefix_command, slash_command, user_cooldown = 1)]
pub async fn stats(
    ctx: Context<'_>,
) -> Result<(), Error> {
    ctx.send(|m| {
        m.content(
            format!(
                "**Version Info**\n\n**Bot:** {bot_version}\n**libavacado:** {avacado_version}\n**Git Commit:** {commit}\n**Commit Message:** {commit_msg}\n**Built On:** {build_cpu}", 
                bot_version = crate::VERSION,
                avacado_version = libavacado::VERSION,
                commit = libavacado::GIT_SHA,
                commit_msg = libavacado::GIT_COMMIT_MSG,
                build_cpu = libavacado::BUILD_CPU,
            )
        )
    }).await?;
    Ok(())
}

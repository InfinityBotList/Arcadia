type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

#[poise::command(category = "Stats", prefix_command, slash_command, user_cooldown = 1)]
pub async fn stats(
    ctx: Context<'_>,
) -> Result<(), Error> {
    ctx.send(|m| {
        m.content(
            format!(
                "
**Version Info**
                
**Bot:** {bot_version}
**libavacado:** {avacado_version} (semver={semver})
**rustc:** {rustc_version}
**Git Commit:** {commit}
**Commit Message:** {commit_msg}
**Built On:** {build_cpu}
**Cargo Profile:** {cargo_profile}
", 
                bot_version = crate::VERSION,
                avacado_version = libavacado::VERSION,
                semver = libavacado::GIT_SEMVER,
                commit = libavacado::GIT_SHA,
                commit_msg = libavacado::GIT_COMMIT_MSG,
                build_cpu = libavacado::BUILD_CPU,
                cargo_profile = libavacado::CARGO_PROFILE,
                rustc_version = libavacado::RUSTC_VERSION,
            )
        )
    }).await?;
    Ok(())
}

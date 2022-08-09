type Error = crate::Error;
type Context<'a> = crate::Context<'a>;


#[poise::command(category = "Stats", prefix_command, slash_command, user_cooldown = 1)]
pub async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(|m| {
        m.embed(|e| {
            e.title("Bot Stats")
            .field("Bot version", crate::VERSION, true)
            .field("libavacado version", libavacado::VERSION, true)
            .field("rustc", libavacado::RUSTC_VERSION, true)
            .field("Git Commit", libavacado::GIT_SHA.to_string() + "(semver=" + libavacado::GIT_SEMVER + ")", true)
            //.field("Uptime", format!("{}", chrono::Duration::from_std(std::time::SystemTime::now().duration_since(start_time)).unwrap()), true)
            .field("Commit Message", libavacado::GIT_COMMIT_MSG, true)
            .field("Built On", libavacado::BUILD_CPU, true)
            .field("Cargo Profile", libavacado::CARGO_PROFILE, true)
        })
    })
    .await?;
    Ok(())
}

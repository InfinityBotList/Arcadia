use crate::_checks as checks;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

#[poise::command(category = "Tests", prefix_command, check = "checks::is_staff")]
pub async fn test_staffcheck(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("You are staff! This check works").await?;
    Ok(())
}

#[poise::command(category = "Tests", prefix_command, check = "checks::is_admin_hdev")]
pub async fn test_admin_dev(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("You are admin or a head dev! This check works")
        .await?;
    Ok(())
}

#[poise::command(category = "Tests", prefix_command, check = "checks::is_admin")]
pub async fn test_admin(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("You are admin or a dev! This check works").await?;
    Ok(())
}

#[poise::command(category = "Tests", prefix_command, check = "checks::is_admin")]
pub async fn test_poll(
    ctx: Context<'_>,
    mut user_list: Vec<poise::serenity_prelude::UserId>,
) -> Result<(), Error> {
    // Ensure that the user list always contains the author
    user_list.push(ctx.author().id);

    let poll_data = crate::_utils::create_vote(ctx, "Test poll", user_list).await?;

    ctx.say(format!("{:#?}", poll_data)).await?;

    Ok(())
}

use log::info;

/// Tries to check if onboarding is required, returns ``false`` if command should stop
pub async fn handle_onboarding(ctx: crate::Context<'_>, user_id: &str, set_onboard_state: Option<String>) -> Result<bool, crate::Error> {
    if !crate::checks::testing_server(ctx).await? {
        return Err("You are not in the testing server".into());
    }

    let _cmd_name = ctx.command().name;

    info!(_cmd_name);

    let data = ctx.data();

    let onboard_state = if set_onboard_state.is_some() {
        set_onboard_state.unwrap()
    } else {
        let res = sqlx::query!(
            "SELECT staff_onboard_state FROM users WHERE user_id = $1",
            user_id
        )
        .fetch_one(&data.pool)
        .await?;
        
        res.staff_onboard_state
    };

    let onboard_state = onboard_state.as_str();
    
    match onboard_state {
        "pending" => {
            let onboarded = sqlx::query!(
                "SELECT staff_onboarded FROM users WHERE user_id = $1",
                user_id
            )
            .fetch_one(&data.pool)
            .await?;

            if !onboarded.staff_onboarded {
                // Ask for staff onboarding
                ctx.say("**Welcome to Infinity Bot List**\n\nSince you seem new to this place, how about a nice look around?").await?;
                return Ok(false);
            }
            Ok(true)
        },
        _ => Ok(true)
    }
}
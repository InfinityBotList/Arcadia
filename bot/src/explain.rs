#[derive(poise::ChoiceParameter)]
pub enum ExplainOption {
    #[name = "Claim"]
    Claim,
    #[name = "Testing"]
    Testing,
    #[name = "Approve/Deny"]
    ApproveDeny,
    #[name = "Tips"]
    Tips,
}

/// An explaination of how the bot works
#[poise::command(category = "Explain", track_edits, prefix_command, slash_command)]
pub async fn explainme(
    ctx: crate::Context<'_>,
    #[description = "Command"] command: Option<ExplainOption>,
) -> Result<(), crate::Error> {
    let text = match command {
        Some(ExplainOption::Claim) => r#"
**Objective**
- Run ``/claim`` to claim the bot

**Explanation**       
Claiming bots will allow you to approve or deny bots after you review a bot! You should do this before you review a bot. Also, if you ever have to stop reviewing a bot. You can always unclaim the bot and ask another staff member, to help review the rest of this bot for you by doing ``/unclaim``
        "#,
        Some(ExplainOption::Testing) => r#"
**Objective**
- Test the bot according to the rules stated in ``/staffguide``

**Explanation**
Test the bot according to the rules stated in ``/staffguide``. Note that you do not have to test every single command in the bot however *the majority* should work. During onboarding however, you must test each command! Once you have done this progress to using the ``/approve`` or ``/deny`` commands.
        "#,
        Some(ExplainOption::ApproveDeny) => r#"
**Objective**
- Approve or deny the bot using either ``/approve`` or ``/deny``

**Explanation**
Approving or denying a bot will remove the bot from the queue and either approve it (it shows on the home page) or deny it (bot has to be resubmitted to the queue for restesting after issues are resolved).
        "#,
        Some(ExplainOption::Tips) => r#"
1. If you are on mobile, consider setting ``embed`` to false in ``queue`` command.

*Explanation*

Copy-pasting embeds on mobile is not well supported.

2. Don't test *every* command, but test the main commands and functionality of the bot (outside of onboarding)

*Explanation*

Testing every command is very time-consuming. You should test the main commands and functionality of the bot (outside of onboarding where you should test every command and report on it in the feedback)

3. Check the bots description

*Explanation*

Where possible, check the bots description to make sure it is readable and does not abuse character limits without good reason.
        "#,
        None => r#"
Welcome to IBL, ``explainme`` is a easy way to get an explanation on commands.

**The Steps**

These are the steps *in order*

1. ``claim``: Claim the bot
2. ``testing``: Test the bot
3. ``approve/deny``: Approve or deny the bot
        "#,
    }.to_string();

    ctx.say(text).await?;

    Ok(())
}

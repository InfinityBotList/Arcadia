use poise::serenity_prelude::GuildId;

use crate::{checks, impls::perms};

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Staff base command
#[poise::command(
    category = "Staff",
    prefix_command,
    slash_command,
    guild_cooldown = 10,
    subcommands("staff_list", "staff_guildlist", "staff_guildleave")
)]
pub async fn staff(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Some available options are ``staff list``, ``staff guildlist``, ``staff_guildleave``")
        .await?;
    Ok(())
}

#[poise::command(
    rename = "list",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server"
)]
pub async fn staff_list(_: Context<'_>) -> Result<(), Error> {
    Err("This command is currently disabled".into())

    /* TODO: FINISH REWRITING
    // Get list of users with staff flag set to true
    let data = ctx.data();

    let server_id = match ctx.guild_id() {
        Some(server_id) => server_id,
        None => return Err("This command can only be used in a server".into()),
    };

    let positions = sqlx::query!(
        "SELECT id, name FROM staff_positions ORDER BY index ASC"
    )
    .fetch_all(&data.pool)
    .await?;

    let mut select_menus = Vec::<CreateSelectMenuOption>::new();

    for position in positions {
        select_menus.push(
            CreateSelectMenuOption::new(format!("{} ({})", position.name, position.id), position.id)
                .description("View staff member's with this position"),
        );
    }

    // Create select menu
    let msg = ctx
        .send(
            CreateReply::new()
                .content("**Please select a position to view a list of staff members**")
                .components(vec![
                    CreateActionRow::SelectMenu(CreateSelectMenu::new(
                        "Choose a position",
                        CreateSelectMenuKind::String {
                            options: select_menus.clone(),
                        },
                    )),
                    CreateActionRow::Buttons(vec![CreateButton::new("sl:cancel").label("Cancel")]),
                ]),
        )
        .await?
        .into_message()
        .await?;

    // Wait for user to select a staff member
    let interaction = msg
        .await_component_interactions(ctx.serenity_context())
        .author_id(ctx.author().id)
        .timeout(Duration::from_secs(120));

    let mut collect_stream = interaction.stream();

    while let Some(item) = collect_stream.next().await {
        item.defer(&ctx.serenity_context()).await?;

        let id = &item.data.custom_id;

        if id == "sl:cancel" {
            log::info!("Received cancel interaction");
            item.delete_response(ctx.serenity_context()).await?;
            return Ok(());
        }

        // Get select menu value
        let values = match &item.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => values,
            _ => {
                log::info!("Received interaction of wrong type: {:?}", item.data.kind);
                continue;
            }
        };

        let id = match values.get(0) {
            Some(id) => id,
            None => {
                log::info!("Failed to get select menu value");
                continue;
            }
        };

        log::info!("Received interaction: {}", id);

        let user_id = match id.parse::<UserId>() {
            Ok(id) => id,
            Err(_) => {
                log::info!("Failed to parse user_id: {}", id);
                continue;
            }
        };

        let member = {
            let cache_user = ctx.serenity_context().cache.member(server_id, user_id);

            match cache_user {
                Some(user) => user.clone(),
                None => {
                    log::error!("Failed to get user from cache: {}", user_id);
                    continue;
                }
            }
        };

        let msg = EditInteractionResponse::new()
            .content("")
            .embed(
                CreateEmbed::default()
                    .title(format!(
                        "{}'s [{}] information",
                        member.user.name,
                        member.display_name()
                    ))
                    .description("This is the information we have on this staff member")
                    .field("User ID", staff.user_id, true)
                    .field("Permissions", perms, true),
            )
            .components(vec![
                CreateActionRow::SelectMenu(CreateSelectMenu::new(
                    "Choose a staff member",
                    CreateSelectMenuKind::String {
                        options: select_menus.clone(),
                    },
                )),
                CreateActionRow::Buttons(vec![CreateButton::new("sl:cancel").label("Cancel")]),
            ]);

        item.edit_response(ctx.serenity_context(), msg).await?;
    }

    Ok(())
    */
}

/// Get guild list, this is intentionally public
#[poise::command(rename = "guildlist", track_edits, prefix_command, slash_command)]
pub async fn staff_guildlist(ctx: Context<'_>) -> Result<(), Error> {
    let guilds = ctx.serenity_context().cache.guilds();

    let mut guild_list = String::new();

    for guild in guilds.iter() {
        let name = guild
            .name(ctx.serenity_context())
            .unwrap_or_else(|| "Unknown".to_string())
            + " ("
            + &guild.to_string()
            + ")\n";
        guild_list.push_str(&name);
    }

    ctx.say(&guild_list).await?;

    Ok(())
}

/// Leave server
#[poise::command(
    rename = "guildleave",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server"
)]
pub async fn staff_guildleave(
    ctx: Context<'_>,
    #[description = "The guild ID to leave"] guild: String,
) -> Result<(), Error> {
    let user_perms = sqlx::query!(
        "SELECT perms FROM staff_members WHERE user_id = $1",
        ctx.author().id.to_string()
    )
    .fetch_one(&ctx.data().pool)
    .await?;

    if !perms::has_perm(&user_perms.perms, &perms::build("arcadia", "leave_guilds")) {
        return Err("You do not have permission to use this command".into());
    }

    let gid = guild.parse::<GuildId>()?;

    ctx.http().leave_guild(gid).await?;

    ctx.say("Removed guild").await?;

    Ok(())
}

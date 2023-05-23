use futures_util::StreamExt;
use poise::{
    serenity_prelude::{
        ComponentInteractionDataKind, CreateActionRow, CreateButton, CreateEmbed, CreateSelectMenu,
        CreateSelectMenuKind, CreateSelectMenuOption, EditInteractionResponse, GuildId,
    },
    CreateReply,
};

use std::{fmt::Write as _, num::NonZeroU64, time::Duration};
// import without risk of name clashing
use poise::serenity_prelude::UserId;

use crate::checks;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Staff base command
#[poise::command(
    category = "Staff",
    prefix_command,
    slash_command,
    guild_cooldown = 10,
    subcommands(
        "staff_list",
        "staff_overview",
        "staff_guildlist",
        "staff_guilddel",
        "staff_guildleave"
    )
)]
pub async fn staff(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Some available options are ``staff list``, ``staff overview``, ``staff guildlist`` (dev/admin only), ``staff_guilddel`` (dev/admin only), ``staff_guildleave`` (dev/admin only), ``staff recalc`` (dev/admin only), ``staff add`` (dev/admin only) etc.").await?;
    Ok(())
}

#[poise::command(
    rename = "list",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server"
)]
pub async fn staff_list(ctx: Context<'_>) -> Result<(), Error> {
    // Get list of users with staff flag set to true
    let data = ctx.data();

    let server_id = match ctx.guild_id() {
        Some(server_id) => server_id,
        None => return Err("This command can only be used in a server".into()),
    };

    let staffs = sqlx::query!(
        "SELECT user_id, staff, admin, ibldev, iblhdev, hadmin, owner FROM users WHERE staff = true ORDER BY user_id ASC"
    )
    .fetch_all(&data.pool)
    .await?;

    if staffs.len() > 25 {
        return Err(
            "Too many staff members to display, please use the ``staff overview`` command instead."
                .into(),
        );
    }

    let mut select_menus = Vec::<CreateSelectMenuOption>::new();

    for staff in staffs {
        let highest_perm = {
            if staff.owner {
                "Owner [owner]"
            } else if staff.hadmin {
                "Head Staff Manager [hadmin]"
            } else if staff.iblhdev {
                "Head Developer [iblhdev]"
            } else if staff.ibldev {
                "Developer [ibldev]"
            } else if staff.admin {
                "Staff Manager [admin]"
            } else {
                "Staff [staff]"
            }
        };

        let user_id = match staff.user_id.parse::<NonZeroU64>() {
            Ok(user_id) => user_id,
            Err(e) => {
                log::error!("Failed to parse user_id: {}", e);
                return Err("Failed to parse user_id".into());
            }
        };

        let cache_user = ctx.discord().cache.member(server_id, UserId(user_id));

        let user = match cache_user {
            Some(user) => user.user,
            None => {
                log::error!("Failed to get user from cache: {}", staff.user_id);
                continue;
            }
        };

        select_menus.push(
            CreateSelectMenuOption::new(format!("{} ({})", user.name, highest_perm), staff.user_id)
                .description("View staff member's information"),
        );
    }

    // Create select menu
    let msg = ctx
        .send(
            CreateReply::new()
                .content("**Please select a staff member to view their information**")
                .components(vec![
                    CreateActionRow::SelectMenu(CreateSelectMenu::new(
                        "Choose a staff member",
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
        .await_component_interactions(ctx.discord())
        .author_id(ctx.author().id)
        .timeout(Duration::from_secs(120));

    let mut collect_stream = interaction.stream();

    while let Some(item) = collect_stream.next().await {
        item.defer(&ctx.discord()).await?;

        let id = &item.data.custom_id;

        if id == "sl:cancel" {
            log::info!("Received cancel interaction");
            item.delete_response(ctx.discord()).await?;
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

        let user_id = match id.parse::<NonZeroU64>() {
            Ok(id) => id,
            Err(_) => {
                log::info!("Failed to parse user_id: {}", id);
                continue;
            }
        };

        let cache_user = ctx.discord().cache.member(server_id, UserId(user_id));

        let member = match cache_user {
            Some(user) => user,
            None => {
                log::error!("Failed to get user from cache: {}", user_id);
                continue;
            }
        };

        let staff = sqlx::query!(
            "SELECT user_id, staff, admin, ibldev, iblhdev, hadmin, owner FROM users WHERE user_id = $1",
            user_id.to_string()
        )
        .fetch_one(&data.pool)
        .await?;

        let perms = {
            let mut perms = "".to_string();

            let errors = {
                let mut errs = Vec::new();
                if staff.hadmin {
                    errs.push(writeln!(perms, "- Head Staff Manager [hadmin]"));
                }
                if staff.iblhdev {
                    errs.push(writeln!(perms, "- Head Developer [iblhdev]"));
                }
                if staff.ibldev {
                    errs.push(writeln!(perms, "- Developer [ibldev]"));
                }
                if staff.admin {
                    errs.push(writeln!(perms, "- Staff Manager [admin]"));
                }
                if staff.staff {
                    errs.push(writeln!(perms, "- Staff [staff]"));
                }
                if staff.owner {
                    errs.push(writeln!(perms, "- Owner [owner]"));
                }

                errs
            };

            for err in errors {
                if let Err(e) = err {
                    log::error!("Failed to write to perms: {}", e);
                    continue;
                }
            }

            perms
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

        item.edit_response(ctx.discord(), msg).await?;
    }

    Ok(())
}

#[poise::command(
    rename = "overview",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::staff_server"
)]
pub async fn staff_overview(ctx: Context<'_>) -> Result<(), Error> {
    // Get list of users with staff flag set to true
    let data = ctx.data();
    let discord = &ctx.discord();

    let staffs = sqlx::query!(
        "SELECT user_id, staff, admin, ibldev, iblhdev, hadmin, owner FROM users WHERE staff = true ORDER BY user_id ASC"
    )
    .fetch_all(&data.pool)
    .await?;

    let mut staff_list = "**Staff List**\n".to_string();

    let guild = ctx.guild().ok_or("Failed to find staff server")?.id;

    for staff in staffs {
        // Convert ID to u64
        let user_id = staff.user_id.parse::<NonZeroU64>()?;

        let cache_user = discord.cache.member(guild, UserId(user_id));

        let user = match cache_user {
            Some(user) => user.user,
            None => {
                return Err(format!("User <@{}> is staff but not in the server", user_id).into());
            }
        };

        writeln!(
            staff_list,
            "{user_id} ({username}) [staff={staff}, admin={admin}, ibldev={ibldev}, iblhdev={iblhdev} hadmin={hadmin}, owner={owner}]", 
            user_id=staff.user_id,
            username=user.name,
            staff=staff.staff,
            admin=staff.admin,
            ibldev=staff.ibldev,
            iblhdev=staff.iblhdev,
            hadmin=staff.hadmin,
            owner=staff.owner
        )?;
    }

    ctx.say(staff_list).await?;

    Ok(())
}

/// Get guild list, this is intentionally public
#[poise::command(rename = "guildlist", track_edits, prefix_command, slash_command)]
pub async fn staff_guildlist(ctx: Context<'_>) -> Result<(), Error> {
    let guilds = ctx.discord().cache.guilds();

    let mut guild_list = String::new();

    for guild in guilds.iter() {
        let name = guild
            .name(ctx.discord())
            .unwrap_or_else(|| "Unknown".to_string())
            + " ("
            + &guild.to_string()
            + ")\n";
        guild_list.push_str(&name);
    }

    ctx.say(&guild_list).await?;

    Ok(())
}

/// Delete server
#[poise::command(
    rename = "guilddel",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_admin_hdev",
    check = "checks::staff_server"
)]
pub async fn staff_guilddel(
    ctx: Context<'_>,
    #[description = "The guild ID to remove"] guild: String,
) -> Result<(), Error> {
    let gid = guild.parse::<NonZeroU64>()?;

    ctx.discord().http.delete_guild(GuildId(gid)).await?;

    ctx.say("Removed guild").await?;

    Ok(())
}

/// Leave server
#[poise::command(
    rename = "guildleave",
    track_edits,
    prefix_command,
    slash_command,
    check = "checks::is_admin_hdev",
    check = "checks::staff_server"
)]
pub async fn staff_guildleave(
    ctx: Context<'_>,
    #[description = "The guild ID to leave"] guild: String,
) -> Result<(), Error> {
    let gid = guild.parse::<NonZeroU64>()?;

    ctx.discord().http.leave_guild(GuildId(gid)).await?;

    ctx.say("Removed guild").await?;

    Ok(())
}

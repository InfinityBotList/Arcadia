use crate::{checks, config};
use poise::serenity_prelude::{RoleId, CacheHttp, GuildId};

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Get your roles based on the bots you own
#[poise::command(
    category = "Bot Owner",
    prefix_command,
    slash_command,
    user_cooldown = 1,
    check = "checks::main_server"
)]
pub async fn getbotroles(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    let id = ctx.author().id.to_string();
    let member = ctx.author_member().await;

    if let Some(member) = member {
        let owned_bots = crate::impls::utils::get_owned_by(&id, &data.pool).await?;

        if owned_bots.is_empty() {
            return Err("You are not the owner/additional owner of any bots".into());
        }

        let mut approved = false;
        let mut certified = false;

        for bot in owned_bots {
            if bot.bot_type == "approved" {
                approved = true; // We need to check for certified as well
                continue;
            }

            if bot.bot_type == "certified" {
                approved = true;
                certified = true;
                break; // No need to check for more
            }
        }

        if !approved {
            return Err(
                "You are not the owner/additional owner of any approved or certified bots".into(),
            );
        }

        let mut roles_to_add = Vec::new();
        let mut roles_to_remove = Vec::new();

        let bot_role = RoleId(config::CONFIG.roles.bot_developer);
        let certified_role = RoleId(config::CONFIG.roles.certified_developer);

        if certified {
            ctx.say(
                "You are the owner/additional owner of a certified bot! Giving you certified role",
            )
            .await?;

            // Check that they have botdev_role, if not, add
            if !member.roles.contains(&bot_role) {
                roles_to_add.push(bot_role);
            }

            if !member.roles.contains(&certified_role) {
                roles_to_add.push(certified_role);
            }
        } else if approved {
            ctx.say(
                "You are the owner/additional owner of an approved bot! Giving you approved role",
            )
            .await?;

            // Check that they have botdev_role, if not, add
            if !member.roles.contains(&bot_role) {
                roles_to_add.push(bot_role);
            }

            if member.roles.contains(&certified_role) {
                roles_to_remove.push(certified_role);
            }
        }

        // Apply the required changes
        if !roles_to_add.is_empty() {
            for role in roles_to_add {
                ctx.http().add_member_role(
                    GuildId(config::CONFIG.servers.main),
                    member.user.id,
                    role,
                    Some("Autorole due to bots owned")
                ).await?;    
            }
        }

        if !roles_to_remove.is_empty() {
            for role in roles_to_remove {
                ctx.http().remove_member_role(
                    GuildId(config::CONFIG.servers.main),
                    member.user.id,
                    role,
                    Some("Autorole due to bots owned")
                ).await?;    
            }
        }

        ctx.say("Done!").await?;
    } else {
        return Err("You are not in the server".into());
    }

    Ok(())
}

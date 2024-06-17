use crate::config;
use crate::{checks, impls::utils::get_user_perms};
use kittycat::perms;
use poise::serenity_prelude::{Color, CreateEmbed, CreateMessage};
use poise::CreateReply;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Let's see who's been fighting bots the most.
#[poise::command(
    category = "Leaderboard",
    rename = "leaderboard",
    prefix_command,
    slash_command
)]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Limit the amount of results."] limit: Option<i64>,
) -> Result<(), Error> {
    let data = ctx.data();
    let number = limit.unwrap_or(5);

    let stats = sqlx::query!(
        "SELECT user_id, approved_count, denied_count FROM (SELECT rpc.user_id, SUM(CASE WHEN rpc.method = 'Approve' THEN 1 ELSE 0 END) AS approved_count, SUM(CASE WHEN rpc.method = 'Deny' THEN 1 ELSE 0 END) AS denied_count, SUM(CASE WHEN rpc.method IN ('Approve', 'Deny') THEN 1 ELSE 0 END) AS total_count FROM rpc_logs rpc LEFT JOIN staff_members sm ON rpc.user_id = sm.user_id WHERE rpc.method IN ('Approve', 'Deny') AND sm.user_id IS NOT NULL GROUP BY rpc.user_id) AS subquery WHERE total_count > 0 ORDER BY total_count DESC LIMIT $1;", 
        number
    )
    .fetch_all(&data.pool)
    .await?;

    let mut desc =
        String::from("Oh, hello there! Let's see who's been fighting bots the most :eyes:\n\n");
    let mut embed = CreateEmbed::default()
        .title("Staff Leaderboard")
        .color(Color::from_rgb(0, 255, 0))
        .description(desc.clone());

    for (index, stat) in stats.iter().enumerate() {
        let emoji = match index {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "",
        };

        desc.push_str(&format!(
            "{} <@{}> | **Approved: {}** | **Denied: {}**\n",
            emoji,
            stat.user_id,
            stat.approved_count.unwrap_or_default(),
            stat.denied_count.unwrap_or_default()
        ));
    }

    embed = embed.description(desc);

    let msg = CreateReply::default().embed(embed);

    ctx.send(msg).await?;
    Ok(())
}

/// Force Refresh Staff Top Reviewers Role
#[poise::command(
    category = "Leaderboard",
    rename = "refresh",
    prefix_command,
    slash_command,
    check = "checks::staff_server"
)]
pub async fn refresh(ctx: Context<'_>) -> Result<(), Error> {
    let user_perms = get_user_perms(&ctx.data().pool, &ctx.author().id.to_string())
        .await?
        .resolve();

    if !perms::has_perm(&user_perms, &"arcadia.force_refresh_top".into()) {
        return Err("You do not have permission to use this command".into());
    }

    let data = ctx.data();
    let pool = &data.pool;

    // Fetch statistics from the database
    let stats = sqlx::query!(
        "SELECT user_id, approved_count, denied_count, total_count FROM (SELECT rpc.user_id, SUM(CASE WHEN rpc.method = 'Approve' THEN 1 ELSE 0 END) AS approved_count, SUM(CASE WHEN rpc.method = 'Deny' THEN 1 ELSE 0 END) AS denied_count, SUM(CASE WHEN rpc.method IN ('Approve', 'Deny') THEN 1 ELSE 0 END) AS total_count FROM rpc_logs rpc LEFT JOIN staff_members sm ON rpc.user_id = sm.user_id WHERE rpc.method IN ('Approve', 'Deny') AND sm.user_id IS NOT NULL GROUP BY rpc.user_id) AS subquery WHERE total_count > 0 ORDER BY total_count DESC LIMIT 3",
    )
    .fetch_all(pool)
    .await?;

    // Get the guild to access its members
    let guild_id = config::CONFIG.servers.main;
    let guild = match ctx.cache().guild(guild_id) {
        Some(guild) => guild.clone(), // Clone the guild data
        None => {
            println!("Failed to get guild");
            return Ok(());
        }
    };

    // Iterate over each member of the guild
    for member in guild.members.iter() {
        // Check if the member has the specified role
        if member.roles.contains(&config::CONFIG.roles.top_reviewers) {
            // Remove the role from the member
            if let Err(why) = member
                .remove_role(
                    ctx.http(),
                    config::CONFIG.roles.top_reviewers,
                    Some("Force syncing top reviewers"),
                )
                .await
            {
                println!(
                    "Failed to remove role from member {}: {:?}",
                    member.user.name, why
                );
            }
        }
    }

    // Get top reviewers from db, and add their roles.
    for stat in stats.iter() {
        let user_id = match stat.user_id.parse::<u64>() {
            Ok(id) => id,
            Err(_) => {
                println!("Failed to parse user_id: {}", stat.user_id);
                continue;
            }
        };

        // Check if the user is in the main server
        if let Ok(member) = guild.member(ctx.http(), user_id.into()).await {
            if let Err(why) = member
                .add_role(
                    ctx.http(),
                    config::CONFIG.roles.top_reviewers,
                    Some("Force syncing top reviewers"),
                )
                .await
            {
                println!("Failed to add role to user {}: {:?}", user_id, why);
            }
        } else {
            continue;
        }
    }

    // Send message into the mod_logs channel to show that the task has been completed!
    let msg = CreateMessage::new().content("**Force Refresh**\nSynced Top Reviewers!");
    let reply = CreateReply::new().content("**Force Refresh**\nSynced Top Reviewers!");

    crate::config::CONFIG
        .channels
        .mod_logs
        .send_message(ctx.http(), msg)
        .await?;

    ctx.send(reply).await?;

    Ok(())
}

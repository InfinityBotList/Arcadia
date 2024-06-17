use crate::config;
use poise::serenity_prelude::CacheHttp;
use poise::serenity_prelude::CreateMessage;

pub async fn topreviewersync(ctx: &serenity::client::Context) -> Result<(), crate::Error> {
    let data = ctx.data::<crate::Data>();
    let pool = &data.pool;

    // Fetch statistics from the database
    let stats = sqlx::query!(
        "SELECT user_id, approved_count, denied_count, total_count FROM (SELECT rpc.user_id, SUM(CASE WHEN rpc.method = 'Approve' THEN 1 ELSE 0 END) AS approved_count, SUM(CASE WHEN rpc.method = 'Deny' THEN 1 ELSE 0 END) AS denied_count, SUM(CASE WHEN rpc.method IN ('Approve', 'Deny') THEN 1 ELSE 0 END) AS total_count FROM rpc_logs rpc LEFT JOIN staff_members sm ON rpc.user_id = sm.user_id WHERE rpc.method IN ('Approve', 'Deny') AND sm.user_id IS NOT NULL GROUP BY rpc.user_id) AS subquery WHERE total_count > 0 ORDER BY total_count DESC LIMIT 3",
    )
    .fetch_all(pool)
    .await?;

    // Get the guild to access its members
    let guild_id = config::CONFIG.servers.main;
    let guild = match ctx.cache.guild(guild_id) {
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
                    Some("Syncing top reviewers, weekly job."),
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
                    Some("Syncing top reviewers, weekly job."),
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
    let msg = CreateMessage::new().content("**Weekly Job**\nSynced Top Reviewers!");

    crate::config::CONFIG
        .channels
        .mod_logs
        .send_message(ctx.http(), msg)
        .await?;

    Ok(())
}

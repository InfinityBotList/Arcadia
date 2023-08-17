use std::str::FromStr;

use futures_util::StreamExt;
use log::{info, warn};
use sqlx::Row;
use strum::VariantNames;
use strum_macros::{EnumVariantNames, EnumString};

#[derive(EnumVariantNames, EnumString)]
enum Entity {
    Bot,
    Server,
    Team,
    Pack,
}

impl Entity {
    fn table_name(&self) -> &'static str {
        match self {
            Entity::Bot => "bots",
	    Entity::Server => "servers",
            Entity::Team => "teams",
            Entity::Pack => "packs",
        }
    }

    fn id_column(&self) -> &'static str {
        match self {
            Entity::Bot => "bot_id",
	    Entity::Server => "server_id",
            Entity::Team => "id",
            Entity::Pack => "url",
        }
    }

    fn target_type(&self) -> &'static str {
        match self {
            Entity::Bot => "bot",
	    Entity::Server => "server",
            Entity::Team => "team",
            Entity::Pack => "pack",
        }
    }
}

pub async fn generic_cleaner(
    pool: &sqlx::PgPool,
) -> Result<(), crate::Error> {
    let mut table_names = sqlx::query!(
        "select table_name from information_schema.columns where column_name = 'target_id'"
    )
    .fetch(pool);

    // table_names is a stream, loop over it
    while let Some(item) = table_names.next().await {
        let item = item?;

        if let Some(table) = item.table_name {
            info!("Validating generic table {}", table);

            clean_table(pool, &table).await?;
        }
    }

    Ok(())
}

async fn clean_table(
    pool: &sqlx::PgPool,
    table: &str,
) -> Result<(), crate::Error> {
    // Fetch target_id and target_type for all in the table such that it does not exist in the corresponding entity table
    for entity in Entity::VARIANTS {

        let Ok(e) = Entity::from_str(entity) else {
            warn!("Invalid entity type {}", entity);
            continue;
        };

        let sql = {
            let table_name = e.table_name();
            let id_column = e.id_column();
            let target_type = e.target_type();
            format!("select target_id, target_type from {table} where target_type = '{target_type}' and not exists (select 1 from {table_name} where {id_column}::text = target_id)")
        };

        let mut rows = sqlx::query(
            &sql,
        )
        .fetch(pool);

        // rows is a stream, loop over it
        while let Some(item) = rows.next().await {
            let item = item.map_err(|e| format!("Error fetching rows: {:?} {}", e, sql))?;

            // Get target_id and target_type from PgRow
            let target_id = item.try_get::<String, &str>("target_id").map_err(|e| format!("Error getting target_id: {:?}", e))?;
            let target_type = item.try_get::<String, &str>("target_type").map_err(|e| format!("Error getting target_type: {:?}", e))?;

            info!("Found orphaned generic entity with table={table}, target_id={target_id}, target_type={target_type}");

            // Delete orphaned generic entity
            let sql = format!("delete from {table} where target_id = $1 and target_type = $2");

            sqlx::query(&sql)
                .bind(&target_id)
                .bind(&target_type)
                .execute(pool)
                .await
                .map_err(|e| format!("Error deleting orphaned generic entity: {:?}", e))?;

            info!("Deleted orphaned generic entity with table={}, target_id={}, target_type={}", table, target_id, target_type);
        }
    }

    Ok(())
}

use log::{error, info, warn};

pub async fn asset_cleaner(ctx: &serenity::all::Context) -> Result<(), crate::Error> {
    let data = ctx.data::<crate::Data>();
    let pool = &data.pool;

    let type_id_map = indexmap::indexmap! {
        "bots" => "bot_id",
        "servers" => "server_id",
        "teams" => "id",
        "partners" => "id",
        "tickets" => "id",
    };

    let assets = ["avatars", "banners", "blobs"];

    let cdn_scopes = crate::config::CONFIG.panel.cdn_scopes.get();
    let Some(cdn_path) = cdn_scopes.get(&crate::config::CONFIG.panel.main_scope) else {
        return Err("No CDN scope for main scope".into());
    };

    // Enumerate over every possbility
    for asset in assets {
        for (entity_type, id_column) in &type_id_map {
            let entity_type_dir = format!("{}/{}/{}", cdn_path.path, asset, entity_type);

            if let Err(e) = std::fs::metadata(&entity_type_dir) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    error!("Could not validate '{}': {}", entity_type_dir, e);
                }
                continue;
            }

            info!("Validating '{}' for entity type '{}'", asset, entity_type);

            let dir = std::fs::read_dir(&entity_type_dir)?;

            for entry in dir {
                let entry = entry?;
                let is_dir = entry.file_type()?.is_dir();
                let file_name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Invalid file name")?; // TODO: Better error handling (maybe
                let file_path = entry.path();

                let Some(id) = file_name.split('.').next() else {
                    warn!("Invalid file name: {}", file_name);

                    if is_dir {
                        std::fs::remove_dir_all(&file_path)?;
                    } else {
                        std::fs::remove_file(&file_path)?;
                    }

                    continue;
                };

                let query = format!(
                    "SELECT {}::text FROM {} WHERE {}::text = $1::text",
                    id_column, entity_type, id_column
                );
                let id: Option<String> = sqlx::query_scalar(&query)
                    .bind(id)
                    .fetch_optional(pool)
                    .await?;

                if id.is_none() {
                    warn!("Found orphaned file: {}", file_path.display());

                    if is_dir {
                        std::fs::remove_dir_all(&file_path)?;
                    } else {
                        std::fs::remove_file(&file_path)?;
                    }
                }
            }
        }
    }

    Ok(())
}

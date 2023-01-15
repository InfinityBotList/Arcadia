use std::io::Read;
use std::{fs::File, sync::Arc};

use crate::types::Error;

use serde::{Deserialize, Serialize};
use serenity::http::CacheHttp;

use rand::{distributions::Alphanumeric, Rng};

// Private struct to handle rust trait errors
pub struct AvcCacheHttpImpl {
    cache: Arc<serenity::cache::Cache>,
    http: Arc<serenity::http::Http>,
}

impl CacheHttp for AvcCacheHttpImpl {
    fn http(&self) -> &serenity::http::Http {
        &self.http
    }

    fn cache(&self) -> Option<&Arc<serenity::cache::Cache>> {
        Some(&self.cache)
    }
}

// Public avacado client used to store caches
pub struct AvacadoPublic {
    pub cache: Arc<serenity::cache::Cache>,

    // Http is unused right now but will be used later
    #[allow(dead_code)]
    pub http: Arc<serenity::http::Http>,

    // Custom struct to avoid rust trait errors
    pub cache_http: AvcCacheHttpImpl,
}

impl AvacadoPublic {
    pub fn new(cache: Arc<serenity::cache::Cache>, http: Arc<serenity::http::Http>) -> Self {
        Self {
            cache: cache.clone(),
            http: http.clone(),
            cache_http: AvcCacheHttpImpl { cache, http },
        }
    }
}

/// Returns a random string of length ``length``
pub fn gen_random(length: usize) -> String {
    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();

    s
}

#[derive(Serialize, Deserialize)]
pub struct Maint {
    pub title: String,
    pub description: String,
    pub done: bool,
}

/// Maintenance status
pub fn maint_status() -> Result<Vec<Maint>, Error> {
    // Open maint.json if itt exists
    let mut maint_file = File::open("/arcmaint.json")?;

    let mut contents = String::new();

    maint_file.read_to_string(&mut contents)?;

    let maint: Vec<Maint> = serde_json::from_str(&contents)?;

    Ok(maint)
}

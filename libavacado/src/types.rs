use serde::Serialize;
use std::sync::Arc;
use serenity::http::CacheHttp;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Debug)]
pub struct ApproveResponse {
    pub invite: String,
}

// Private struct to handle rust trait errors
#[derive(Debug, Clone)]
pub struct CacheHttpImpl {
    pub cache: Arc<serenity::cache::Cache>,
    pub http: Arc<serenity::http::Http>,
}

impl CacheHttp for CacheHttpImpl {
    fn http(&self) -> &serenity::http::Http {
        &self.http
    }

    fn cache(&self) -> Option<&Arc<serenity::cache::Cache>> {
        Some(&self.cache)
    }
}
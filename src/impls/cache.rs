use std::sync::Arc;

use poise::serenity_prelude::{Cache, CacheHttp, Http};

/// A Simple struct that implements the CacheHttp trait because serenity can't seem to keep this stable
#[derive(Debug, Clone)]
pub struct CacheHttpImpl {
    pub cache: Arc<Cache>,
    pub http: Arc<Http>,
}

impl CacheHttp for CacheHttpImpl {
    fn http(&self) -> &Http {
        &self.http
    }

    fn cache(&self) -> Option<&Arc<Cache>> {
        Some(&self.cache)
    }
}

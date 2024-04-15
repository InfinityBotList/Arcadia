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

impl CacheHttpImpl {
    pub fn from_ctx(ctx: &serenity::all::Context) -> Self {
        Self {
            cache: ctx.cache.clone(),
            http: ctx.http.clone(),
        }
    }
}

impl From<(Arc<Cache>, Arc<Http>)> for CacheHttpImpl {
    fn from(c: (Arc<Cache>, Arc<Http>)) -> Self {
        Self {
            cache: c.0,
            http: c.1,
        }
    }
}

impl From<serenity::all::Context> for CacheHttpImpl {
    fn from(c: serenity::all::Context) -> Self {
        Self {
            cache: c.cache,
            http: c.http,
        }
    }
}

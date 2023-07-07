use async_trait::async_trait;
use futures_util::TryStreamExt;
use mongodb::{bson::Document, options::FindOptions};
use serde::Deserialize;

use crate::{Data, Error};

#[async_trait]
pub trait Model {
    fn collection_name() -> &'static str;

    async fn get(data: &Data, filter: Document, opts: Option<FindOptions>) -> Result<Vec<Self>, Error>
    where
        Self: Sync + Sized,
        for<'de> Self: Deserialize<'de>,
    {
        let collection = data
            .mongo
            .database(Self::collection_name())
            .collection(Self::collection_name());

        let result = collection.find(filter, opts).await?;

        Ok(result.try_collect().await?)
    }
}

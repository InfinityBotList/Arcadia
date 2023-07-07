use super::core::Model;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct InternalCases {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    /// The unique ID of the document in the database
    pub id: Option<ObjectId>,

    /// The user ID of the user who the case was created for
    pub user: String,

    /// The guild ID of the guild where the case was created
    pub guild: String,

    /// The reason for the case
    pub reason: String,

    /// The action taken for the case
    pub action: String,

    /// The moderator ID of the moderator who created the case
    pub moderator: String,

    /// The case ID
    pub case_id: u64,

    /// The time of the case (?)
    pub time: String,

    /// The duration of the case (?)
    pub duration: String,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created: chrono::DateTime<chrono::Utc>,
}

impl Model for InternalCases {
    fn collection_name() -> &'static str {
        "internal_cases"
    }
}

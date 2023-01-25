use serde::{Serialize};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Debug)]
pub struct ApproveResponse {
    pub invite: String
}
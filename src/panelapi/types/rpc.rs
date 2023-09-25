use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::{
    impls::target_types::TargetType,
    rpc::core::{RPCField, RPCPerms},
};

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = ".generated/RPCWebAction.ts")]
pub struct RPCWebAction {
    /// ID of the RPC action
    pub id: String,
    /// Label of the RPC action
    pub label: String,
    /// Description of the RPC action
    pub description: String,
    /// Fields of the RPC action
    pub fields: Vec<RPCField>,
    /// Target types supported by the RPC action
    pub supported_target_types: Vec<TargetType>,
    /// Permissions required to use the RPC action
    pub needs_perms: RPCPerms,
}

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// MFA Login Secret Data
#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/MfaLoginSecret.ts")]
pub struct MfaLoginSecret {
    pub secret: String,
    pub otp_url: String,
    pub qr_code: String,
}

/// MFA Login Data
#[derive(Serialize, Deserialize, TS, ToSchema, Clone)]
#[ts(export, export_to = ".generated/MfaLogin.ts")]
pub struct MfaLogin {
    pub info: Option<MfaLoginSecret>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = ".generated/AuthData.ts")]
pub struct AuthData {
    pub user_id: String,
    pub created_at: i64,
    pub state: String,
}
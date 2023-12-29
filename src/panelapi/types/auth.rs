use serde::{Deserialize, Serialize};
use ts_rs::TS;
use strum_macros::{Display, EnumString, EnumVariantNames};
use utoipa::ToSchema;

#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    TS,
    EnumString,
    EnumVariantNames,
    Display,
    Clone,
    PartialEq,
)]
#[ts(export, export_to = ".generated/AuthorizeAction.ts")]
pub enum AuthorizeAction {
    /// Begin begins an authorization request
    /// 
    /// Currently only returns a scope and the login url
    Begin {
        /// Scope of the panel. This is a short identifier to ensure a valid arcadia instance
        scope: String,
        /// Redirect URL
        redirect_url: String,
    },
    
    /// CreateSession creates a new 'pending' session for the staff member returning a login token
    /// 
    /// Note that MFA/other login methods are needed to 'activate' the session
    CreateSession {
        /// Discord OAuth2 code
        code: String,
        /// Redirect URL
        redirect_url: String,
    },

    /// CheckMFA checks and returns any needed/useful MFA-related information
    /// 
    /// This is the only endpoint that works on both pending and active sessions
    CheckMfaState {
        /// Login Token
        login_token: String,
    },

    /// Resets MFA for a user identified by login token
    ResetMfaTotp {
        /// Login token
        login_token: String,
        /// Old MFA code
        otp: String,
    },

    /// ActivateSession activates a session for a given login token
    ActivateSession {
        /// Login token
        login_token: String,
        /// MFA code
        otp: String,
    },

    /// Logout logs out a session
    Logout {
        /// Login token
        login_token: String,
    },
}

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

#[derive(Serialize, Deserialize, TS, Clone)]
#[ts(export, export_to = ".generated/AuthData.ts")]
pub struct AuthData {
    pub user_id: String,
    pub created_at: i64,
    pub state: String,
}

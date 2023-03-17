use strum_macros::{Display, EnumString};

/// Only covers the specific onboarding fields that we care about
#[derive(PartialEq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum OnboardState {
    Pending,
    PendingManagerReview,
    Denied,
    Completed,
}
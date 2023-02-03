#[derive(PartialEq)]
pub enum OnboardState {
    Pending,
    QueueRemind,
    QueueForceClaim,
    Claimed,
    PendingManagerReview,
    Denied,
    Completed,
}

impl OnboardState {
    pub fn as_str(&self) -> &str {
        match self {
            OnboardState::Pending => "pending",
            OnboardState::QueueRemind => "queue-remind",
            OnboardState::QueueForceClaim => "queue-force-claim",
            OnboardState::Claimed => "claimed",
            OnboardState::PendingManagerReview => "pending-manager-review",
            OnboardState::Denied => "denied",
            OnboardState::Completed => "completed",
        }
    }

    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            "pending" => Some(OnboardState::Pending),
            "queue-remind" => Some(OnboardState::QueueRemind),
            "queue-force-claim" => Some(OnboardState::QueueForceClaim),
            "claimed" => Some(OnboardState::Claimed),
            "pending-manager-review" => Some(OnboardState::PendingManagerReview),
            "denied" => Some(OnboardState::Denied),
            "completed" => Some(OnboardState::Completed),
            _ => None,
        }
    }

    pub fn queue_unclaim(&self) -> bool {
        match self {
            OnboardState::Pending => true,
            OnboardState::QueueRemind => true,
            OnboardState::QueueForceClaim => true,
            _ => false,
        }
    }

    pub fn queue_passthrough(&self) -> bool {
        match self {
            OnboardState::Pending => true,
            OnboardState::PendingManagerReview => true,
            OnboardState::Denied => true,
            OnboardState::Completed => true,
            _ => false,
        }
    }
}

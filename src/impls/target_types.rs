use std::fmt::{Formatter, Display};

#[derive(PartialEq)]
pub enum TargetType {
    Bot,
    Server,
    Team,
    Pack,
}

impl Display for TargetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Bot => write!(f, "bot"),
            TargetType::Server => write!(f, "server"),
            TargetType::Team => write!(f, "team"),
            TargetType::Pack => write!(f, "pack"),
        }
    }
}

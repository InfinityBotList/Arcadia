/// Poise doesn't seem to handle this anymore
#[derive(poise::ChoiceParameter)]
pub enum Bool {
    #[name = "True"]
    True,
    #[name = "False"]
    False,
}

impl Bool {
    pub fn to_bool(&self) -> bool {
        match self {
            Bool::True => true,
            Bool::False => false,
        }
    }
}

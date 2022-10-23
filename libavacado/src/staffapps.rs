use crate::types::{StaffAppData, StaffPosition, StaffAppQuestion};

pub fn get_apps() -> StaffAppData {
    StaffAppData {
        positions: vec!["staff".to_string(), "dev".to_string()],
        staff: StaffPosition {
            open: true,
            name: "Staff Team".to_string(),
            info: r#"Join the Infinity Staff Team and help us Approve, Deny and Certify Discord Bots. 

We are a welcoming and laid back team who is always willing to give new people an opportunity!"#.to_string(),
            questions: vec![
                StaffAppQuestion {
                    id: "experience".to_string(),
                    question: "Past server experience".to_string(),
                    para: "Tell us any experience you have working for other servers or bot lists.".to_string(),
                    placeholder: "I have worked at...".to_string(),
                },
                StaffAppQuestion {
                    id: "strengths".to_string(),
                    question: "List some of your strengths".to_string(),
                    para: "Tell us a little bit about yourself.".to_string(),
                    placeholder: "I am always online and active...".to_string(),
                },
                StaffAppQuestion {
                    id: "situations".to_string(),
                    question: "Situation Examples".to_string(),
                    para: "How would you handle: Mass Pings, Nukes and Raids etc.".to_string(),
                    placeholder: "I would handle it by...".to_string(),
                },
                StaffAppQuestion {
                    id: "reason".to_string(),
                    question: "Why do you want to join the staff team?".to_string(),
                    para: "Why do you want to join the staff team? Be specific".to_string(),
                    placeholder: "I want to join the staff team because...".to_string(),
                },
                StaffAppQuestion {
                    id: "other".to_string(),
                    question: "Anything else you want to add?".to_string(),
                    para: "Anything else you want to add?".to_string(),
                    placeholder: "Just state anything that doesn't hit anywhere else".to_string(),
                },
            ],
        },
        dev: StaffPosition {
            open: true,
            name: "Dev Team".to_string(),
            info: "Join our Dev Team and help us Update, Manage and Maintain all of the Infinity Services!".to_string(),
            questions: vec![
                StaffAppQuestion {
                    id: "experience".to_string(),
                    question: "Do you have experience in Javascript, Rust and/or Golang. Give examples of projects/code you have written".to_string(),
                    para: "Do you have experience in Javascript, Rust and/or Golang. Give examples of projects/code you have written.".to_string(),
                    placeholder: "I have worked on...".to_string(),
                },
                StaffAppQuestion {
                    id: "strengths".to_string(),
                    question: "What are your strengths in coding Javascript, Rust and/or Golang.".to_string(),
                    para: "What are your strengths in coding Javascript, Rust and/or Golang.".to_string(),
                    placeholder: "I am good at...".to_string(),
                },
                StaffAppQuestion {
                    id: "db".to_string(),
                    question: "Do you have Exprience with PostgreSQL".to_string(),
                    para: "Do you have Exprience with PostgreSQL".to_string(),
                    placeholder: "I have used PostgreSQL for...".to_string(),
                },
                StaffAppQuestion {
                    id: "reason".to_string(),
                    question: "Why do you want to join the dev team?".to_string(),
                    para: "Why do you want to join the dev team? Be specific".to_string(),
                    placeholder: "I want to join the dev team because...".to_string(),
                },
            ]
        }
    }
}
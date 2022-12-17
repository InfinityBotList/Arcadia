use crate::public::AvacadoPublic;
use crate::types::{Error, StaffAppData, StaffAppQuestion, StaffAppResponse, StaffPosition};
use serde_json::json;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::model::prelude::UserId;
use serenity::prelude::Mentionable;
use sqlx::PgPool;
use std::collections::HashMap;
use std::num::NonZeroU64;

pub fn get_apps() -> StaffAppData {
    StaffAppData {
        positions: vec!["staff".to_string(), "dev".to_string(), "certification".to_string(), "partners".to_string()],
        staff: StaffPosition {
            open: true,
            interview: Some(vec![
                StaffAppQuestion {
                    id: "motive".to_string(),
                    question: "Why did you apply for the staff team position?".to_string(),
                    para: "Why did you apply for this role? Be specific. We want to know how you can help Infinity Bot List and why you wish to".to_string(),
                    placeholder: "I applied because...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "team-player".to_string(),
                    question: "What is a scenario in which you had to be a team player?".to_string(),
                    para: "What is a scenario in which you had to be a team player? We want to know that you can collaborate effectively with us.".to_string(),
                    placeholder: "I had to...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "about-you".to_string(),
                    question: "Tell us a little about yourself".to_string(),
                    para: "Tell us a little about yourself. Its that simple!".to_string(),
                    placeholder: "I am...".to_string(),
                    short: false,
                }        
            ]),
            app_site_rendered: true,
            name: "Staff Team".to_string(),
            info: r#"Join the Infinity Staff Team and help us Approve, Deny and Certify Discord Bots. 

We are a welcoming and laid back team who is always willing to give new people an opportunity!"#.to_string(),
            questions: vec![
                StaffAppQuestion {
                    id: "experience".to_string(),
                    question: "Past server experience".to_string(),
                    para: "Tell us any experience you have working for other servers or bot lists.".to_string(),
                    placeholder: "I have worked at...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "strengths".to_string(),
                    question: "List some of your strengths".to_string(),
                    para: "Tell us a little bit about yourself.".to_string(),
                    placeholder: "I am always online and active...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "situations".to_string(),
                    question: "Situation Examples".to_string(),
                    para: "How would you handle: Mass Pings, Nukes and Raids etc.".to_string(),
                    placeholder: "I would handle it by...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "reason".to_string(),
                    question: "Why do you want to join the staff team?".to_string(),
                    para: "Why do you want to join the staff team? Be specific".to_string(),
                    placeholder: "I want to join the staff team because...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "other".to_string(),
                    question: "Anything else you want to add?".to_string(),
                    para: "Anything else you want to add?".to_string(),
                    placeholder: "Just state anything that doesn't hit anywhere else".to_string(),
                    short: true,
                },
            ],
        },
        dev: StaffPosition {
            open: true,
            interview: Some(vec![
                StaffAppQuestion {
                    id: "motive".to_string(),
                    question: "Why did you apply for the dev team position?".to_string(),
                    para: "Why did you apply for this role? Be specific. We want to know how you can help Infinity Bot List and why you wish to".to_string(),
                    placeholder: "I applied because...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "team-player".to_string(),
                    question: "What is a scenario in which you had to be a team player?".to_string(),
                    para: "What is a scenario in which you had to be a team player? We want to know that you can collaborate effectively with us.".to_string(),
                    placeholder: "I had to...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "some-work".to_string(),
                    question: "What is some of the projects you have done? Can you share some links with us?".to_string(),
                    para: "What is some of the projects you have done? Can you share some links with us? We want to see your finest works".to_string(),
                    placeholder: "Some work I did...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "about-you".to_string(),
                    question: "Tell us a little about yourself".to_string(),
                    para: "Tell us a little about yourself. Its that simple!".to_string(),
                    placeholder: "I am...".to_string(),
                    short: false,
                },
            ]),
            app_site_rendered: true,
            name: "Dev Team".to_string(),
            info: r#"Join our Dev Team and help us update, manage and maintain all of the Infinity Services!
            
Experience in PostgreSQL and at least one of the below languages is required:

- Rust
- TypeScript (Javascript with type-safety)
- Go
"#.to_string(),
            questions: vec![
                StaffAppQuestion {
                    id: "experience".to_string(),
                    question: "Do you have experience in Typescript, Rust and/or Golang. Give examples of projects/code you have written".to_string(),
                    para: "Do you have experience in Typescript, Rust and/or Golang. Give examples of projects/code you have written.".to_string(),
                    placeholder: "I have worked on...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "strengths".to_string(),
                    question: "What are your strengths in coding".to_string(),
                    para: "What are your strengths in coding".to_string(),
                    placeholder: "I am good at...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "db".to_string(),
                    question: "Do you have Exprience with PostgreSQL. How much experience do you have?".to_string(),
                    para: "Do you have Exprience with PostgreSQL".to_string(),
                    placeholder: "I have used PostgreSQL for... and know...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "sql".to_string(),
                    question: "Write a SQL expression to select apples, bananas and kiwis from a table named fruits".to_string(),
                    para: "Tell us a little about yourself. Its that simple!".to_string(),
                    placeholder: "I am...".to_string(),
                    short: false,
                },                
                StaffAppQuestion {
                    id: "reason".to_string(),
                    question: "Why do you want to join the dev team?".to_string(),
                    para: "Why do you want to join the dev team? Be specific".to_string(),
                    placeholder: "I want to join the dev team because...".to_string(),
                    short: false,
                },
            ]
        },
        certification: StaffPosition {
            open: true,
            interview: None,
            app_site_rendered: true, // For now, until this is decided
            name: "Bot Certification".to_string(),
            info: r#"
Certify your discord bot for extra perks and more!

Fill out this form and it will be added on our certification app queue.
"#.to_string(),
            questions: vec![
                StaffAppQuestion {
                    id: "id".to_string(),
                    question: "What is your bots ID?".to_string(),
                    para: "What do you feel is unique about your bot? This could be anything!".to_string(),
                    placeholder: "My bot does...".to_string(),
                    short: true,
                },
                StaffAppQuestion {
                    id: "unique".to_string(),
                    question: "What do you feel is unique about your bot?".to_string(),
                    para: "What do you feel is unique about your bot? This could be anything!".to_string(),
                    placeholder: "My bot does...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "gain".to_string(),
                    question: "What do you hope to gain through certification".to_string(),
                    para: "What special feature/perk do you want to gain through certification! What do you believe your bot can bring for our services?".to_string(),
                    placeholder: "I hope to gain...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "features".to_string(),
                    question: "What features of Infinity Bot List does your bot use (posting stats/banners on bot page etc.)?".to_string(),
                    para: "This doesn't account for much weightage but we want to know how much functionality and perks you already use".to_string(),
                    placeholder: "I use...".to_string(),
                    short: false,
                },
            ],
        },
        partners: StaffPosition {
            open: true,
            interview: None,
            app_site_rendered: true,
            name: "Partners".to_string(),
            info: r#"Partner your Discord Bot, Discord Server or Business today! It's easier than ever before!"#.to_string(),
            questions: vec![
                StaffAppQuestion {
                    id: "what".to_string(),
                    question: "What are you looking to partner with us for?".to_string(),
                    para: "What are you looking to partner with us for? Be descriptive here".to_string(),
                    placeholder: "I wish to partner a bot/website called Foobar because...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "why".to_string(),
                    question: "Why do you want to partner with us?".to_string(),
                    para: "Why do you want to partner with us? Be specific".to_string(),
                    placeholder: "I want to partner with Infinity Bot List because...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "how".to_string(),
                    question: "How will you promote us?".to_string(),
                    para: "How will you promote Infinity Bot List? This could be a partner command or a link on your website!".to_string(),
                    placeholder: "I will promote Infinity Bot List using...".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "demo".to_string(),
                    question: "Do you have anything to showcase what you wish to partner with us?".to_string(),
                    para: "Links to show us demos of what you're partnering or how many members your server or bot has.".to_string(),
                    placeholder: "LINK 1 etc.".to_string(),
                    short: false,
                },
                StaffAppQuestion {
                    id: "other".to_string(),
                    question: "Anything else you want to add?".to_string(),
                    para: "Anything else you want to add?".to_string(),
                    placeholder: "Just state anything that doesn't hit anywhere else".to_string(),
                    short: true,
                },
            ],
        },
    }
}

pub async fn get_app_interview(
    pool: &PgPool,
    user_id: &str,
    app_id: &str,
) -> Result<Vec<StaffAppQuestion>, Error> {
    let row = sqlx::query!("SELECT state, position FROM apps WHERE app_id = $1 AND user_id = $2", app_id, user_id)
        .fetch_one(pool)
        .await;
    
    if row.is_err() {
        return Err("Error fetching application".into());
    }

    let row = row.unwrap();

    if row.state != "pending-interview" {
        return Err("This application is not pending an interview".into())
    }

    let app_questions = get_apps();

    let position = app_questions.staff_questions(&row.position);

    if position.interview.is_none() {
        return Err("This position does not need an interview".into())
    }

    Ok(position.interview.as_ref().unwrap().to_vec())
}

pub async fn create_app(
    public: &AvacadoPublic,
    pool: &PgPool,
    user_id: &str,
    position_id: &str,
    app: HashMap<String, String>,
) -> Result<(), Error> {
    let user_apps = sqlx::query!(
        "SELECT COUNT(1) FROM apps WHERE user_id = $1 AND position = $2 AND (state = 'pending' OR state = 'pending-interview' OR state = 'pending-approval')",
        user_id,
        position_id,
    )
    .fetch_one(pool)
    .await?;

    if user_apps.count.unwrap_or(0) > 0 {
        return Err("You already have a pending application for this position".into());
    }

    let app_questions = get_apps();

    let position = app_questions.staff_questions(position_id);

    if !position.open {
        return Err("This position is currently closed".into());
    }

    let mut app_map = HashMap::new();
    for question in &position.questions {
        // Get question from app.
        let answer = app.get(&question.id).ok_or("Missing question")?;

        // Check if answer is empty.
        if answer.is_empty() {
            return Err("An answer you have sent is empty".into());
        }

        // Check if answer is too short.
        if answer.len() < 50 && !question.short {
            return Err("An answer you have sent is too short".into());
        }

        // Add answer to map.
        app_map.insert(&question.id, answer.to_string());
    }

    let app_id = crate::public::gen_random(128);

    // Create app
    sqlx::query!(
        "INSERT INTO apps (app_id, user_id, position, answers) VALUES ($1, $2, $3, $4)",
        app_id,
        user_id,
        position_id,
        json!({
            "questions": position.questions,
            "answers": app_map
        }),
    )
    .execute(pool)
    .await?;

    // Send a message to the APPS channel
    let user_id = UserId(user_id.parse::<NonZeroU64>()?);

    let app_channel = std::env::var("APP_CHANNEL_ID")?;

    let app_channel = ChannelId(app_channel.parse::<NonZeroU64>()?);

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("New Application")
            .description(format!(
                "{} has applied for the {} position.",
                user_id.mention(),
                position_id
            ))
            .field("User ID", user_id.to_string(), false)
            .field("Position", position_id, false)
            .field(
                "Answers (For right now, to allow testing)",
                "https://ptb.botlist.app/testview/".to_string() + &app_id,
                false,
            )
            .url("https://ptb.infinitybots.gg/apps/view/".to_string() + &app_id),
    );

    app_channel.send_message(&public.http, msg).await?;

    Ok(())
}

pub async fn send_interview(
    public: &AvacadoPublic,
    pool: &PgPool,
    app_id: &str,
) -> Result<(), Error> {
    let row = sqlx::query!("SELECT user_id, state, position FROM apps WHERE app_id = $1", app_id,)
        .fetch_one(pool)
        .await?;

    if row.state != "pending" {
        return Err("This application is not in the 'pending' state".into());
    }

    let app_questions = get_apps();

    let position = app_questions.staff_questions(&row.position);

    if position.interview.is_none() {
        return Err("This position does not need an interview".into());
    }

    sqlx::query!(
        "UPDATE apps SET state = 'pending-interview' WHERE app_id = $1",
        app_id
    )
    .execute(pool)
    .await?;

    // Send a message to the APPS channel
    let user_id = UserId(row.user_id.parse::<NonZeroU64>()?);

    let app_channel = std::env::var("APP_CHANNEL_ID")?;

    let app_channel = ChannelId(app_channel.parse::<NonZeroU64>()?);

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("User Selected For Interview")
            .description(format!(
                "{} has been selected for an interview for the {} position.",
                user_id.mention(),
                &row.position
            ))
            .field("User ID", user_id.to_string(), false)
            .field("Position", &row.position, false)
            .field(
                "Answers",
                "https://ptb.botlist.app/testview/".to_string() + &app_id,
                false,
            )
            .url("https://ptb.infinitybots.gg/apps/view/".to_string() + &app_id),
    );

    app_channel.send_message(&public.http, msg).await?;

    // Create DM channel
    let dm = user_id.create_dm_channel(&public.http).await;

    if dm.is_err() {
        println!("Failed to send DM to {}", row.user_id);
        return Err("Failed to send DM to user [dm channel create failed]".into());
    }

    let dm = dm.unwrap();

    let msg = CreateMessage::new().embed(
        CreateEmbed::default()
            .title("Interview")
            .description(format!(
                "You have been selected for an interview for the {} position. [Click here]({})",
                &row.position,
                "https://ptb.botlist.app/interview/".to_string() + &app_id
            ))
            .url("https://ptb.botlist.app/interview/".to_string() + &app_id),
    );

    let resp = dm.send_message(&public.http, msg).await;

    if resp.is_err() {
        println!("Failed to send DM to {}", row.user_id);
        return Err("Failed to send DM to user".into());
    }

    Ok(())
}

pub async fn finalize_app(
    public: &AvacadoPublic,
    pool: &PgPool,
    user_id: &str,
    app_id: &str,
    interview: HashMap<String, String>,
) -> Result<(), Error> {
    let row = sqlx::query!(
        "SELECT state, position FROM apps WHERE app_id = $1 AND user_id = $2",
        app_id,
        user_id,
    )
    .fetch_one(pool)
    .await?;

    let app_questions = get_apps();

    let position = app_questions.staff_questions(&row.position);

    if !position.interview.is_none() {
        if row.state != "pending" {
            return Err("This application is not in the 'pending' state".into());
        }

        sqlx::query!(
            "UPDATE apps SET state = 'approved' WHERE app_id = $1",
            app_id
        )
        .execute(pool)
        .await?;
    } else {
        if row.state != "pending-interview" {
            return Err("This application is not in the 'pending-interview' state".into());
        }

        let questions = position.interview.as_ref().unwrap();

        let mut app_map = HashMap::new();
        for question in questions {
            // Get question from app.
            let answer = interview.get(&question.id).ok_or("Missing question")?;

            // Check if answer is empty.
            if answer.is_empty() {
                return Err("An answer you have sent is empty".into());
            }

            // Check if answer is too short.
            if answer.len() < 50 {
                return Err("An answer you have sent is too short".into());
            }

            // Add answer to map.
            app_map.insert(&question.id, answer.to_string());
        }

        sqlx::query!(
            "UPDATE apps SET state = 'pending-approval', interview_answers = $1 WHERE app_id = $2",
            json!({
                "questions": questions,
                "answers": app_map
            }),
            app_id
        )
        .execute(pool)
        .await?;
    }

    // Send a message to the APPS channel
    let user_id = UserId(user_id.parse::<NonZeroU64>()?);

    let app_channel = std::env::var("APP_CHANNEL_ID")?;

    let app_channel = ChannelId(app_channel.parse::<NonZeroU64>()?);

    let msg = CreateMessage::default().embed(
        CreateEmbed::default()
            .title("Application Finalized")
            .description(format!(
                "{} has been finalized their application with an interview.",
                user_id.mention()
            ))
            .field("User ID", user_id.mention().to_string(), false)
            .field("Position", &row.position, false)
            .field(
                "Answers",
                "https://ptb.botlist.app/testview/".to_string() + &app_id,
                false,
            )
            .url("https://ptb.botlist.app/testview/".to_string() + &app_id),
    );

    app_channel.send_message(&public.http, msg).await?;

    Ok(())
}

pub async fn get_made_apps(pool: &PgPool) -> Result<Vec<StaffAppResponse>, Error> {
    let mut apps = Vec::new();

    let apps_db = sqlx::query!("SELECT app_id, user_id, created_at, state, answers, interview_answers, likes, dislikes, position FROM apps")
        .fetch_all(pool)
        .await?;

    for app in apps_db {
        let mut likes = Vec::new();

        for like in app.likes {
            likes.push(like.to_string());
        }

        let mut dislikes = Vec::new();

        for dislike in app.dislikes {
            dislikes.push(dislike.to_string());
        }

        apps.push(StaffAppResponse {
            app_id: app.app_id,
            user_id: app.user_id,
            created_at: app.created_at,
            state: app.state,
            answers: app.answers,
            interview: app.interview_answers,
            position: app.position,
            likes,
            dislikes,
        });
    }

    Ok(vec![])
}

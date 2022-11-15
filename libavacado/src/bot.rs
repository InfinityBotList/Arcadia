use log::info;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{types::{Error, CreateBot}, public::{AvacadoPublic, get_user}};

use itertools::Itertools;

use serde_json::json;
use pulldown_cmark::{html::push_html, Options, Parser};

pub fn sanitize(
    text: &str,
) -> String {
    // Parse to HTML
    let options = Options::all();
    let md_parse = Parser::new_ext(text, options);
    let mut html = String::new();
    push_html(&mut html, md_parse);

    ammonia::Builder::new()
        .rm_clean_content_tags(&["style", "iframe"])
        .add_tags(&[
            "span", "img", "video", "iframe", "style", "p", "br", "center", "div", "h1", "h2",
            "h3", "h4", "h5", "section", "article", "lang",
        ])
        .add_generic_attributes(&[
            "id",
            "class",
            "style",
            "data-src",
            "data-background-image",
            "data-background-image-set",
            "data-background-delimiter",
            "data-icon",
            "data-inline",
            "data-height",
            "code",
        ])
        .add_tag_attributes("iframe", &["src", "height", "width"])
        .add_tag_attributes(
            "img",
            &[
                "src",
                "alt",
                "width",
                "height",
                "crossorigin",
                "referrerpolicy",
                "sizes",
                "srcset",
            ],
        )
        .clean(&html)
        .to_string()
}

#[derive(Deserialize)]
struct JapiApp {
    cached: bool,
    data: JapiAppData,
}

#[derive(Deserialize)]
struct JapiAppData {
    application: JapiAppDataApplication,
    bot: JapiAppDataBot,
}

#[derive(Deserialize)]
struct JapiAppDataApplication {
    id: String,
    bot_public: bool,
}

#[derive(Deserialize)]
struct JapiAppDataBot {
    id: String,
    approximate_guild_count: i64,
}

pub async fn check_bot_client_id(bot: &mut CreateBot) -> Result<(), Error> {
    let req = reqwest::get(format!("https://japi.rest/discord/v1/application/{}", bot.client_id))
        .await?;
    
    if req.status() == 429 {
        return Err(
            format!(
                "Whoa there! We're being ratelimited by our anti-abuse provider! Please try again in {} seconds.", 
                req.headers().get("retry-after").map_or("unknown", |v| v.to_str().unwrap_or("unknown"))
            ).into()
        );
    } else if !req.status().is_success() {
        return Err(
            format!(
                "We couldn't find a bot with that client ID! Status code: {}",
                req.status()
            ).into()
        );
    }

    // Note that warnings are not handled here, as they are not fatal to adding a bot
    let japi_app: JapiApp = req.json().await?;

    if !japi_app.data.application.bot_public {
        return Err("This bot is not public!".into());
    }

    if !japi_app.cached {
        info!("JAPI cache MISS for {}", bot.client_id);
    } else {
        info!("JAPI cache HIT for {}", bot.client_id);
    }

    // This check exists to ensure we can put the API check after the database and user check
    if bot.bot_id != japi_app.data.bot.id || bot.client_id != japi_app.data.application.id {
        return Err("The bot ID provided does not match the bot ID found!".into());
    }

    bot.guild_count = japi_app.data.bot.approximate_guild_count;

    Ok(())
}

pub async fn add_bot(
    public: &AvacadoPublic,
    pool: &PgPool, 
    main_owner: &str,
    bot: &mut CreateBot
) -> Result<(), Error> {
    // Put the simple checks first

    // Ensure main owner is not in additional owners
    if bot.additional_owners.contains(&main_owner.to_string()) {
        return Err("Whoa there! The main owner of this bot is also in the additional owners list".into());
    }

    // Ensure maximum of 7 additional owners
    if bot.additional_owners.len() > 7 {
        return Err("Whoa there! You can only have 7 additional owners".into());
    }

    // Ensure short is between 50 and 150 characters
    if bot.short.len() < 50 || bot.short.len() > 150 {
        return Err("Whoa there! The short description must be between 50 and 150 characters long".into());
    }

    // Ensure long description is at least 500 characters
    if bot.long.len() < 500 {
        return Err("Whoa there! The long description must be at least 500 characters long".into());
    }

    // If prefix is empty, set it to /
    if bot.prefix.is_empty() {
        bot.prefix = "/".to_string();
    }

    // Ensure prefix is shorter than 10 characters
    if bot.prefix.len() > 10 {
        return Err("Whoa there! The prefix must be 10 characters or less".into());
    }

    // Ensure all extra links are HTTPS, this needs special scoping
    {
        let mut private = 0;
        let mut public = 0;
        for (name, link) in &bot.extra_links {
            if name.starts_with("_") {
                // Private link, don't validate HTTPS
                private += 1;

                if link.len() > 8192 {
                    return Err("Whoa there! One of your private links is too long".into());
                }

                // this only applies to private links
                if link.replace(' ', "").is_empty() {
                    return Err("Whoa there! One of your private links is empty".into());
                }

                continue;
            }

            public += 1;

            if !link.starts_with("https://") {
                return Err(("Whoa there! Extra link (".to_string() + name + ") must be HTTPS").into());
            }

            if link.len() > 512 {
                return Err("Whoa there! One of your extra links is too long".into());
            }
        }

        if private > 10 {
            return Err("Whoa there! You can only have 10 private extra links".into());
        }

        if public > 10 {
            return Err("Whoa there! You can only have 10 public extra links".into());
        }
    }

    // Ensure invite is HTTPS
    if !bot.invite.starts_with("https://") {
        return Err("Whoa there! The invite must be HTTPS".into());
    }

    // Ensure background is HTTPS
    if !bot.background.starts_with("https://") {
        return Err("Whoa there! The background must be HTTPS".into());
    }

    // Ensure tags are not empty
    if bot.tags.is_empty() {
        return Err("Whoa there! You must have at least one tag".into());
    }

    // Ensure there is a maximum of 5 tags
    if bot.tags.len() > 5 {
        return Err("Whoa there! You can only have 5 tags".into());
    }

    // Ensure there are no duplicate tags
    if bot.tags.len() != bot.tags.iter().unique().count() {
        return Err("Whoa there! You cannot have duplicate tags".into());
    }

    // Ensure tags are not longer than 20 characters
    for tag in &bot.tags {
        if tag.len() > 20 {
            return Err("Whoa there! Tags must be 20 characters or less".into());
        }
    }

    // More complex checks in terms of resources

    // Ensure bot isn't already in the database
    let bot_exists = sqlx::query!("SELECT EXISTS(SELECT 1 FROM bots WHERE bot_id = $1)", bot.bot_id)
        .fetch_one(pool)
        .await?
        .exists;

    // If it is, return an error, we are conservative here, if this returns None then something is wrong
    if bot_exists.unwrap_or(true) {
        return Err("Bot already exists in the database (or something badly went wrong!)".into());
    }

    // Ensure the bot actually exists
    let bot_user = get_user(public, &bot.bot_id, true).await?;

    if !bot_user.valid {
        return Err("Whoa there! This bot does not exist".into());
    }

    if !bot_user.bot {
        return Err("Whoa there! This user is not a bot!".into());
    }
    
    // Ensure the owner exists
    let owner_user = get_user(public, main_owner, true).await?;

    if !owner_user.valid {
        return Err("Whoa there! The main owner of this bot does not exist".into());
    }

    if owner_user.bot {
        return Err("Whoa there! The main owner of this bot is a bot!".into());
    }

    // Ensure that additional owners exist
    for owner in &bot.additional_owners {
        let owner_user = get_user(public, owner, true).await?;

        if !owner_user.valid {
            return Err(("Whoa there! Additional owner (".to_string() + owner + ") of this bot does not exist").into());
        }

        if owner_user.bot {
            return Err(("Whoa there! Additional owner (".to_string() + owner + ") of this bot is a bot!").into());
        }
    }

    // Validate the bot IDs API side as well to prevent any client hacks
    check_bot_client_id(bot).await?;

    // Now we can insert the bot into the database
    sqlx::query!(
        "INSERT INTO bots (bot_id, client_id, owner, additional_owners, short, long, prefix, invite, extra_links, tags, library, nsfw, cross_add, approval_note, banner) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        bot.bot_id,
        bot.client_id,
        main_owner,
        &bot.additional_owners,
        bot.short,
        bot.long,
        bot.prefix,
        bot.invite,
        json!(bot.extra_links),
        &bot.tags,
        bot.library,
        bot.nsfw,
        bot.cross_add,
        bot.staff_note,
        bot.background
    )
    .execute(pool)
    .await?;

    Ok(())
}

use sqlx::PgPool;

use crate::{types::{Error, CreateBot}, public::{AvacadoPublic, get_user}};

use itertools::Itertools;

use serde_json::json;

pub fn sanitize(
    text: &str,
) -> String {
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
        .clean(&text)
        .to_string()
}

pub async fn add_bot(
    public: &AvacadoPublic,
    pool: &PgPool, 
    main_owner: &str,
    bot: &mut CreateBot
) -> Result<(), Error> {
    // First ensure bot isn't already in the database
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

    // Ensure the owner exists
    let owner_user = get_user(public, main_owner, true).await?;

    if !owner_user.valid {
        return Err("Whoa there! The main owner of this bot does not exist".into());
    }

    // Ensure maximum of 7 additional owners
    if bot.additional_owners.len() > 7 {
        return Err("Whoa there! You can only have 7 additional owners".into());
    }

    // Ensure that additional owners exist
    for owner in &bot.additional_owners {
        let owner_user = get_user(public, owner, true).await?;

        if !owner_user.valid {
            return Err(("Whoa there! Additional owner (".to_string() + owner + ") of this bot does not exist").into());
        }
    }

    // Ensure main owner is not in additional owners
    if bot.additional_owners.contains(&main_owner.to_string()) {
        return Err("Whoa there! The main owner of this bot is also in the additional owners list".into());
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

    // Ensure all extra links are HTTPS
    let mut private = 0;
    let mut public = 0;
    for (name, link) in &bot.extra_links {
        if name.starts_with("_") {
            // Private link, don't validate HTTPS
            private += 1;

            if link.len() > 8192 {
                return Err("Whoa there! One of your private links is too long".into());
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

    // Now we can insert the bot into the database
    sqlx::query!(
        "INSERT INTO bots (bot_id, owner, additional_owners, short, long, prefix, invite, extra_links, tags, library, nsfw, cross_add, approval_note, banner) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        bot.bot_id,
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
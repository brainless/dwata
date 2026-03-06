use anyhow::{Context, Result};
use clap::Parser;
use dwata_agents::normalize_email_content;
use dwata_api::database::emails as emails_db;
use dwata_api::helpers::database::initialize_database;

#[derive(Debug, Parser)]
#[command(
    name = "inspect_email_content",
    about = "Print original and normalized content for a single email ID"
)]
struct Args {
    /// Email ID from the emails table
    email_id: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let db = initialize_database().context("Failed to initialize database")?;

    let email = emails_db::get_email(db.async_connection.clone(), args.email_id)
        .await
        .with_context(|| format!("Failed to load email id={}", args.email_id))?;

    let original_body =
        preferred_original_body(email.body_text.as_deref(), email.body_html.as_deref());
    let normalized = normalize_email_content(
        email.subject.as_deref(),
        email.body_text.as_deref(),
        email.body_html.as_deref(),
    );

    println!("Email ID: {}", email.id);
    println!("Subject:");
    println!("{}", email.subject.as_deref().unwrap_or_default());
    println!();
    println!("Original Body Source: {}", original_body.0);
    println!("Original Body:");
    println!("{}", original_body.1);
    println!();
    println!("Normalized Subject:");
    println!("{}", normalized.subject);
    println!();
    println!("Normalized Body:");
    println!("{}", normalized.body);

    Ok(())
}

fn preferred_original_body<'a>(
    body_text: Option<&'a str>,
    body_html: Option<&'a str>,
) -> (&'static str, &'a str) {
    if let Some(text) = body_text {
        if !text.trim().is_empty() {
            return ("body_text", text);
        }
    }
    if let Some(html) = body_html {
        return ("body_html", html);
    }
    ("none", "")
}

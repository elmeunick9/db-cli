use serde_json::json;
use std::path::Path;
use crate::utils::fs;
use crate::config::Config;
use tracing::{info, debug, warn};

/// Create a migration plan
pub async fn execute(config: &Config, from_version: Option<String>, to_version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    if !config.is_dev() {
        warn!("Plan command is disabled in production mode.");
        return Ok(());
    }

    let sql_base = config.sql_base();
    let mut from_version = from_version.unwrap_or_else(|| "latest".to_string());
    let mut to_version = to_version.unwrap_or_else(|| "next".to_string());

    // Resolve version aliases
    if from_version == "latest" {
        from_version = fs::get_latest_version(sql_base)?;
    }

    if to_version == "latest" {
        to_version = fs::get_latest_version(sql_base)?;
    }

    info!("Creating migration plan from version '{}' to '{}'", from_version, to_version);

    // Check if versions exist
    let from_dir = Path::new(sql_base).join(&from_version);
    let to_dir = Path::new(sql_base).join(&to_version);
    if !from_dir.exists() {
        return Err(format!("From version directory not found: {}", from_dir.display()).into());
    }
    if !to_dir.exists() {
        return Err(format!("To version directory not found: {}", to_dir.display()).into());
    }

    let diff = fs::diff_dir_recursive(&from_dir, &to_dir, |p| {
        let is_sql_file = p.extension().map_or(false, |ext| ext == "sql");
        let is_allowed_name = p.file_name().and_then(|s| s.to_str()).map_or(false, |name| {
            let parts: Vec<&str> = name.split('.').collect();
            let is_migration_script = parts.len() == 2 && parts[0].parse::<u32>().is_ok();
            let is_test_script = parts.len() == 3 && parts[0].parse::<u32>().is_ok() && parts[2] == "test.sql";

            !is_migration_script && !is_test_script
        });

        is_sql_file && is_allowed_name
    })?;

    debug!("{}", diff);

    // Use LLM to create migration plan
    let migration_sql = generate_migration_plan(config, &diff, &from_version, &to_version).await?;

    // Write migration plan to file
    let migration_file = format!("{}/{}.sql", to_dir.to_string_lossy(), from_version);
    std::fs::write(&migration_file, &migration_sql)?;
    info!("Migration plan written to: {}", migration_file);

    Ok(())
}

async fn generate_migration_plan(
    config: &Config,
    diff: &str,
    from_version: &str,
    to_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if !config.ai.enabled {
        return Err("AI is not enabled in configuration".into());
    }

    let client = reqwest::Client::new();
    
    // Custom instructions for the AI
    let system_instructions = "You are a database expert. Generate a high-quality SQL migration script \
    based on the provided diff. Note that the diff may contain line comments (--). \
    On files with status MODIFIED only partial results may be provided, otherwise the fill content of the file will be provided. \
    Ensure the output contains only the SQL code. \
    Note that you don't need BEGIN/COMMIT statements. \
    Make sure to explicitly specify the schema names in the generated SQL code, e.g. \"public\".\"my_table\".\"my_column\". \
    (folder name structure is /<version>/<schema>/...).";

    let payload = json!({
        "model": config.ai.model,
        "messages": [
            {
                "role": "system",
                "content": system_instructions
            },
            {
                "role": "user",
                "content": format!("Generate a migration from {} to {}. Here is the diff:\n\n{}", from_version, to_version, diff)
            }
        ]
    });

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", config.ai.api_key))
        // OpenRouter often requires these for ranking/identification
        .header("HTTP-Referer", "https://github.com/elmeunick9/db-cli") 
        .header("X-Title", "DB-CLI")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_body = response.text().await?;
        return Err(format!("OpenRouter API error: {}", error_body).into());
    }

    let result: serde_json::Value = response.json().await?;
    
    // Extract the content from the typical OpenAI/OpenRouter response structure
    let ai_content = result["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to parse AI response content")?;

    let ai_content = ai_content.trim();

    // Extraction Logic
    let normalized_content = if ai_content.starts_with("```") {
        // Find the end of the first line (e.g., ```sql\n)
        if let Some(first_newline) = ai_content.find('\n') {
            // Find the last triple backtick
            if let Some(last_backticks) = ai_content.rfind("```") {
                // Ensure the last backticks aren't the ones at the very start
                if last_backticks > first_newline {
                    ai_content[first_newline..last_backticks].trim()
                } else {
                    ai_content
                }
            } else {
                ai_content
            }
        } else {
            ai_content
        }
    } else {
        ai_content
    };

    Ok(format!("{}", normalized_content))
}

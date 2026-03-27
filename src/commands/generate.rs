use crate::config::Config;

pub async fn execute(config: &Config, format: Option<crate::GenerateFormat>, version: Option<String>, output: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("TODO: generate");
    Ok(())
}
use crate::config::Config;
use crate::utils::db::get_db_pool;
use crate::utils::fs;
use crate::utils::inspect;
use crate::utils::write_json::write_json;

pub async fn execute(config: &Config, format: crate::GenerateFormat, version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let sql_base = config.sql_base();
    let target_version = if let Some(v) = version {
        v
    } else {
        fs::get_latest_version(&sql_base)?
    };

    let format_name = format.as_str();
    let output_path = config.generate_output_dir(format_name);
    std::fs::create_dir_all(&output_path)?;

    println!("Generating metadata for version {}", target_version);

    let schemas = fs::list_schemas(&sql_base, &target_version)?;
    if schemas.is_empty() {
        println!("No schemas found under {}/{}", sql_base, target_version);
        return Ok(());
    }

    let pool = get_db_pool(config).await?;

    for schema in schemas {
        println!("Inspecting schema: {}", schema);
        let schema_info = inspect::inspect_schema(&pool, &schema).await?;
        match format {
            crate::GenerateFormat::Json => write_json(&output_path, &schema_info)?,
        }
    }

    Ok(())
}


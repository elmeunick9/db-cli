use crate::config::Config;
use crate::utils::db::get_db_pool;
use crate::utils::fs;
use crate::utils::inspect;

pub async fn execute(config: &Config, format: Option<crate::GenerateFormat>, version: Option<String>, output: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let _ = format;
    let _ = output;
    let sql_base = config.sql_base();
    let target_version = if let Some(v) = version {
        v
    } else {
        fs::get_latest_version(&sql_base)?
    };

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
        for table in schema_info.tables {
            println!("  Table: {}", table.name);
            for column in table.columns {
                println!("    {} {}{}{}{}", column.name, column.data_type, if column.is_nullable { " NULLABLE" } else { " NOT NULL" }, column.default.as_ref().map(|d| format!(" DEFAULT {}", d)).unwrap_or_default(), column.udt_name);
            }
        }
    }

    Ok(())
}
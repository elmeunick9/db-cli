use crate::config::Config;
use crate::utils::db::get_db_pool;
use crate::utils::fs;
use crate::utils::generate;
use crate::utils::inspect;

pub async fn execute(config: &Config, format: String, version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let sql_base = config.sql_base();
    let target_version = if let Some(v) = version {
        v
    } else {
        fs::get_latest_version(&sql_base)?
    };

    let format_name = format.trim();
    let output_path = config.generate_output_dir(format_name);
    let input_path = config.generate_input_dir(format_name);
    std::fs::create_dir_all(&output_path)?;

    println!("Generating metadata for version {}", target_version);

    let schemas = fs::list_schemas(&sql_base, &target_version)?;
    if schemas.is_empty() {
        println!("No schemas found under {}/{}", sql_base, target_version);
        return Ok(());
    }

    let pool = get_db_pool(config).await?;
    let mut schema_infos = Vec::with_capacity(schemas.len());

    for schema in schemas {
        println!("Inspecting schema: {}", schema);
        schema_infos.push(inspect::inspect_schema(&pool, &schema).await?);
    }

    if let Some(input_path) = input_path.as_ref() {
        generate::write_from_template_dir(
            input_path,
            &output_path,
            format_name,
            &target_version,
            &schema_infos,
        )?;
    } else if format_name.eq_ignore_ascii_case("json") {
        generate::write_json_default(&output_path, &schema_infos)?;
    } else {
        return Err(format!("generate format '{}' requires input_dir in db.toml", format_name).into());
    }

    Ok(())
}


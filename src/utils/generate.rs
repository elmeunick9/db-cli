use crate::utils::inspect::SchemaInfo;
use crate::utils::handlebars_helpers;
use crate::utils::rhai_helpers;
use handlebars::Handlebars;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

const TRANSFORM_SCRIPT_NAME: &str = "generate.rhai";

pub fn write_from_template_dir(
    input_dir: &Path,
    output_dir: &Path,
    format_name: &str,
    version: &str,
    schemas: &[SchemaInfo],
) -> Result<(), Box<dyn std::error::Error>> {
    if !input_dir.exists() {
        return Err(format!("generate input_dir does not exist: {}", input_dir.display()).into());
    }

    if !input_dir.is_dir() {
        return Err(format!("generate input_dir is not a directory: {}", input_dir.display()).into());
    }

    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);
    handlebars.set_strict_mode(true);
    handlebars_helpers::register_common_helpers(&mut handlebars);

    let transform_script = load_transform_script(input_dir)?;
    if let Some(script) = transform_script.as_deref() {
        rhai_helpers::register_handlebars_helpers(&mut handlebars, script)?;
    }

    let handlebars = Arc::new(handlebars);
    let context = build_context(schemas, format_name, version)?;

    if let Some(script) = transform_script.as_deref() {
        rhai_helpers::run_main_script(script, &context, input_dir, output_dir, Arc::clone(&handlebars))?;
        copy_static_files(input_dir, output_dir)?;
        return Ok(());
    }

    render_template_tree(handlebars.as_ref(), input_dir, output_dir, &context)
}

pub fn write_json_default(
    output_dir: &Path,
    schemas: &[SchemaInfo],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_dir)?;

    for schema in schemas {
        let file_name = format!("{}.json", schema.schema);
        let file_path = output_dir.join(file_name);
        let content = serde_json::to_string(schema)?;
        fs::write(file_path, content)?;
    }

    Ok(())
}

fn render_template_tree(
    handlebars: &Handlebars<'_>,
    input_dir: &Path,
    output_dir: &Path,
    context: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in WalkDir::new(input_dir).into_iter().filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if !path.is_file() || is_transform_script(path) {
            continue;
        }

        let relative_path = path.strip_prefix(input_dir)?;
        let rendered_relative_path = render_relative_path(&handlebars, relative_path, &context)?;
        let output_file = output_dir.join(rendered_relative_path);

        if let Some(parent) = output_file.parent() {
            fs::create_dir_all(parent)?;
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some("hbs") {
            let template = fs::read_to_string(path)?;
            let rendered = handlebars.render_template(&template, &context)?;
            fs::write(output_file, rendered)?;
        } else {
            fs::copy(path, output_file)?;
        }
    }

    Ok(())
}

fn copy_static_files(input_dir: &Path, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in WalkDir::new(input_dir).into_iter().filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if !path.is_file() || is_transform_script(path) || is_handlebars_template(path) {
            continue;
        }

        let relative_path = path.strip_prefix(input_dir)?;
        let output_file = output_dir.join(relative_path);

        if let Some(parent) = output_file.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(path, output_file)?;
    }

    Ok(())
}

fn build_context(
    schemas: &[SchemaInfo],
    format_name: &str,
    version: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut object = Map::new();
    object.insert("format".to_string(), Value::String(format_name.to_string()));
    object.insert("version".to_string(), Value::String(version.to_string()));
    object.insert("schemas".to_string(), serde_json::to_value(schemas)?);
    Ok(Value::Object(object))
}

fn load_transform_script(input_dir: &Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let script_path = input_dir.join(TRANSFORM_SCRIPT_NAME);
    if !script_path.exists() {
        return Ok(None);
    }

    Ok(Some(fs::read_to_string(&script_path)?))
}

fn render_relative_path(
    handlebars: &Handlebars<'_>,
    relative_path: &Path,
    context: &Value,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut rendered = PathBuf::new();

    for component in relative_path.components() {
        let segment = component.as_os_str().to_string_lossy();
        let mut rendered_segment = handlebars.render_template(&segment, context)?;
        if rendered_segment.ends_with(".hbs") {
            rendered_segment.truncate(rendered_segment.len() - 4);
        }
        rendered.push(rendered_segment);
    }

    Ok(rendered)
}

fn is_transform_script(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some(TRANSFORM_SCRIPT_NAME)
}

fn is_handlebars_template(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("hbs")
}


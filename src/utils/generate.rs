use crate::utils::inspect::SchemaInfo;
use crate::utils::handlebars_helpers;
use crate::utils::rhai_helpers;
use handlebars::Handlebars;
use rhai::serde::{from_dynamic, to_dynamic};
use rhai::{Dynamic, Engine, Scope};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
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

    let context = build_context(schemas, format_name, version)?;
    let context = apply_transform(transform_script.as_deref(), &context)?;

    if let Some(outputs) = context.get("outputs").and_then(Value::as_array) {
        render_declared_outputs(&handlebars, input_dir, output_dir, &context, outputs)?;
        copy_static_files(input_dir, output_dir)?;
        return Ok(());
    }

    render_template_tree(&handlebars, input_dir, output_dir, &context)
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

fn render_declared_outputs(
    handlebars: &Handlebars<'_>,
    input_dir: &Path,
    output_dir: &Path,
    root_context: &Value,
    outputs: &[Value],
) -> Result<(), Box<dyn std::error::Error>> {
    for output in outputs {
        let output_object = output
            .as_object()
            .ok_or("generate outputs entries must be objects")?;
        let context = merge_context(root_context, output_object.get("context"))?;
        let path_template = get_required_string(output_object, "path")?;
        let rendered_relative_path = handlebars.render_template(path_template, &context)?;
        let output_file = output_dir.join(rendered_relative_path);

        if let Some(parent) = output_file.parent() {
            fs::create_dir_all(parent)?;
        }

        if let Some(content_template) = output_object.get("content").and_then(Value::as_str) {
            let rendered = handlebars.render_template(content_template, &context)?;
            fs::write(output_file, rendered)?;
            continue;
        }

        if let Some(template_path) = output_object.get("template").and_then(Value::as_str) {
            let template = fs::read_to_string(input_dir.join(template_path))?;
            let rendered = handlebars.render_template(&template, &context)?;
            fs::write(output_file, rendered)?;
            continue;
        }

        return Err("generate outputs entries must define either 'template' or 'content'".into());
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

fn apply_transform(script: Option<&str>, context: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let Some(script) = script else {
        return Ok(context.clone());
    };

    let engine = Engine::new();
    let ast = engine.compile(script)?;
    let dynamic_context = to_dynamic(context)?;
    let transformed = match engine.call_fn::<Dynamic>(
        &mut Scope::new(),
        &ast,
        "transform",
        (dynamic_context,),
    ) {
        Ok(transformed) => transformed,
        Err(err) if is_missing_rhai_function(&err.to_string(), "transform") => {
            return Ok(context.clone())
        }
        Err(err) => return Err(err.into()),
    };

    Ok(from_dynamic(&transformed)?)
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

fn merge_context(
    root_context: &Value,
    extra_context: Option<&Value>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut merged = root_context
        .as_object()
        .cloned()
        .ok_or("generate root context must be an object")?;

    if let Some(extra_context) = extra_context {
        let extra_object = extra_context
            .as_object()
            .ok_or("generate outputs context must be an object")?;
        for (key, value) in extra_object {
            merged.insert(key.clone(), value.clone());
        }
    }

    Ok(Value::Object(merged))
}

fn get_required_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("generate outputs entries require string field '{}'", key).into())
}

fn is_missing_rhai_function(message: &str, function_name: &str) -> bool {
    message.contains("Function not found") && message.contains(function_name)
}
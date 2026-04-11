use std::path::{Path, PathBuf};

use handlebars::Handlebars;
use rhai::{Dynamic, EvalAltResult, ImmutableString, Scope};
use rhai::serde::{from_dynamic, to_dynamic};
use std::sync::{Arc, Mutex};

type DynError = Box<dyn std::error::Error>;
type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

use crate::config::{Config, GenerateEntry};
use crate::utils::db::get_db_pool;
use crate::utils::fs;
use crate::utils::inspect;

fn rhai_error(message: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(message.into().into(), rhai::Position::NONE).into()
}

fn load_script(input_dir: &Path) -> Result<String, DynError> {
    let script_path = input_dir.join("generate.rhai");
    if !script_path.exists() {
        return Err(format!("transform script not found: {}", script_path.display()).into());
    }

    Ok(std::fs::read_to_string(&script_path)?)
}

async fn generate(config: &Config, format: &GenerateEntry, version: &str) -> Result<(), DynError> {
    std::fs::create_dir_all(&format.output_dir)?;
    println!("Generating code for format {}", format.format);
    
    // 1) Get schema info from DB (Inspect)
    let sql_base = config.sql_base();
    let schemas = fs::list_schemas(&sql_base, &version)?;
    if schemas.is_empty() {
        println!("No schemas found under {}/{}", sql_base, version);
        return Ok(());
    }

    let pool = get_db_pool(config).await?;
    let mut schema_infos = Vec::with_capacity(schemas.len());

    for schema in schemas {
        println!("Inspecting schema: {}", schema);
        schema_infos.push(inspect::inspect_schema(&pool, &schema).await?);
    }

    // 2) Prepare input/output folders and handle special json case
    let output_path = PathBuf::from(format.output_dir.clone());
    if format.format.eq_ignore_ascii_case("json") && format.input_dir.is_none() {
        for schema in &schema_infos {
            let file_name = format!("{}.json", schema.schema);
            let file_path = output_path.join(file_name);
            let content = serde_json::to_string(schema)?;
            std::fs::write(file_path, content)?;
        }
        return Ok(());
    } else if format.input_dir.is_none() {
        return Err(format!("Format '{}' requires input_dir in db.toml", format.format).into());
    }
    let input_path = PathBuf::from(format.input_dir.as_ref().unwrap());
    if !input_path.exists() || !input_path.is_dir(){
        return Err(format!("input_dir does not exist or is not a directory: {}", input_path.display()).into());
    }

    // 3) Load rhai script and register helpers
    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);
    handlebars.set_strict_mode(true);
    crate::utils::handlebars::register_handlebars_helpers(&mut handlebars);

    let script = load_script(&input_path)?;
    let mut engine = rhai::Engine::new();
    let ast = engine.compile(script)?;
    let mut scope = Scope::new();
    let context = serde_json::to_value(&schema_infos)?;
    scope.push_dynamic("context", rhai::serde::to_dynamic(context)?);
    crate::utils::handlebars::register_handlebars_rhai_module(&mut engine);

    let hb_shared = Arc::new(Mutex::new(handlebars));
    let shared_ast = std::sync::Arc::new(ast);

    engine.register_fn(
        "transform",
        {
            let hb_shared = hb_shared.clone(); 
            move |context: Dynamic, template: ImmutableString| -> RhaiResult<ImmutableString> {
                let context_value: serde_json::Value = from_dynamic(&context).map_err(|err| rhai_error(err.to_string()))?;
                let rendered = hb_shared.lock().unwrap()
                    .render_template(template.as_str(), &context_value)
                    .map_err(|err| rhai_error(err.to_string()))?;
                Ok(rendered.into())
            }
        },
    );

    engine.register_fn(
        "load",
        move |path: ImmutableString| -> RhaiResult<ImmutableString> {
            let resolved = fs::ensure_relative_path(&input_path, path.as_str()).map_err(
                |err| rhai_error(err.to_string())
            )?;
            let content = std::fs::read_to_string(&resolved).map_err(|err| rhai_error(err.to_string()))?;
            Ok(content.into())
        },
    );

    engine.register_fn(
        "save",
        move |path: ImmutableString, content: ImmutableString| -> RhaiResult<ImmutableString> {
            let resolved = fs::ensure_relative_path(&output_path, path.as_str()).map_err(
                |err| rhai_error(err.to_string())
            )?;
            if let Some(parent) = resolved.parent() {
                std::fs::create_dir_all(parent).map_err(|err| rhai_error(err.to_string()))?;
            }
            std::fs::write(&resolved, content.as_str()).map_err(|err| rhai_error(err.to_string()))?;
            Ok(content)
        },
    );

    engine.register_fn("register", {
        let shared_ast = shared_ast.clone();
        let hb_shared = hb_shared.clone(); 
        
        move |name: ImmutableString, rhai_fn_name: ImmutableString| -> RhaiResult<()> {
            let mut hb = hb_shared.lock().unwrap();
            let ast = shared_ast.clone();
            
            hb.register_helper(
                &name.to_string(),
                Box::new(move |
                    h: &handlebars::Helper<'_>, 
                    _: &handlebars::Handlebars<'_>, 
                    _: &handlebars::Context, 
                    _: &mut handlebars::RenderContext<'_, '_>, 
                    out: &mut dyn handlebars::Output| -> Result<(), handlebars::RenderError> {
                    
                    let engine = rhai::Engine::new();
                    let args: Vec<rhai::Dynamic> = h.params()
                        .iter()
                        .map(|p| rhai::Dynamic::from(p.value().clone()))
                        .collect();

                    // If rhai_fn_name is defined as 'my_func(a, b)' but args.len() is 3,
                    // Rhai will return an EvalAltResult::ErrorFunctionNotFound.
                    let result: String = engine.call_fn(
                        &mut rhai::Scope::new(), 
                        &ast, 
                        &rhai_fn_name, 
                        args
                    ).map_err(|e| {
                        handlebars::RenderErrorReason::Other(format!(
                            "Rhai function '{}' execution failed: {}", rhai_fn_name, e
                        ))
                    })?;

                    out.write(&result).map_err(|e| handlebars::RenderErrorReason::Other(e.to_string()))?;
                    Ok(())
                }),
            );
            Ok(())
        }
    });

    // 4) Call rhai script main function
    engine
        .call_fn::<Dynamic>(&mut scope, &shared_ast.clone(), "main", ())
        .map(|_| ())
        .map_err(Into::<DynError>::into)?;

    Ok(())
}

pub async fn execute(config: &Config, version: Option<String>, format: Option<String>) -> Result<(), DynError> {
    let pool = get_db_pool(config).await?;
    let sql_base = config.sql_base();
    let mut version = version.unwrap_or_else(|| "next".to_string());

    // Resolve version aliases like "latest" to a concrete version folder
    if version == "latest" {
        match fs::get_latest_version(&sql_base) {
            Ok(v) => {
                version = v;
            }
            Err(e) => {
                return Err(format!("failed to resolve 'latest' version: {}", e).into());
            }
        }
    }

    let mut version_match_required = false;
    if config.is_dev() && config.dry_run {
        tracing::info!("Dry run mode enabled. Will try version match with current DB");
        version_match_required = true;
    }
    if !config.is_dev() {
        version_match_required = true;
    }
    if version_match_required {
        let db_version = crate::commands::apply::get_current_version(&pool, &config).await.unwrap_or_else(|e| {
            eprintln!("Failed to get current DB version: {}", e);
            std::process::exit(1);
        });
        if db_version == version {
            tracing::info!("DB version '{}' matches requested version '{}'", db_version, version);
        } else {
            tracing::info!("DB version '{}' does NOT match requested version '{}'. Aborting.", db_version, version);
            std::process::exit(1);
        }
    } else {
        crate::commands::init::execute(config, Some(version.clone())).await?;
    }

    let format_string = format.unwrap_or_else(|| "all".to_string());
    let formats: Vec<GenerateEntry> = if format_string.eq_ignore_ascii_case("all") {
        config.generate.clone()
    } else {
        format_string.split(',').map(|s| 
            config.generate.iter().find(|e|
                e.format.eq_ignore_ascii_case(s)
            ).unwrap().clone()
        ).collect()
    };
    for format in formats {
        generate(config, &format, &version).await?;
    }
    Ok(())
}
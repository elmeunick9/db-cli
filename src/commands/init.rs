use std::path::{Path};
use crate::utils::db;
use crate::utils::fs;
use crate::utils::blocks;
use crate::config::Config;

/// Initialize the database
pub async fn execute(config: &Config, version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let sql_base = config.sql_base();
    let mut version = version.unwrap_or_else(|| "next".to_string());

    // Resolve version aliases like "latest" to a concrete version folder
    if version == "latest" {
        match fs::get_latest_version(sql_base) {
            Ok(v) => {
                version = v;
            }
            Err(e) => {
                return Err(format!("failed to resolve 'latest' version: {}", e).into());
            }
        }
    }

    println!("Initializing database for version: {}", version);
    println!("Mode: {}", if config.is_dev() { "development" } else { "production" });

    let version_dir = Path::new(sql_base).join(&version);
    if !version_dir.exists() {
        return Err(format!("Version directory not found: {}", version_dir.display()).into());
    }

    // 1) Create DB
    db::create_db(config).await?;

    // // 2) List schemas
    let schemas = fs::list_schemas(sql_base, &version)?;
    println!("Found schemas: {:?}", schemas);

    // // 3) Read all files and parse blocks per schema
    let mut blocks: Vec<blocks::Block> = vec![];
    let mut files_count = 0;
    for schema in &schemas {
        let files = fs::read_schema_files(sql_base, &version, schema)?;
        files_count += files.len();
        for file in files {
            let bks = blocks::parse_blocks(schema, &file)?;
            blocks.extend(bks);
        }
    }

    println!("Found {} files and {} blocks", files_count, blocks.len());

    for block in &blocks {
        if !block.file.ends_with("schema.sql") {
            continue;
        }
        let sql_preview = if block.sql.len() > 50 {
            format!("{}...", &block.sql[..block.sql.len().min(900)])
        } else {
            block.sql.clone()
        };
        println!("-- BLOCK {:?} --", block.name);
        println!("schema={}, file={}, requires={:?}",
                 block.schema, block.file, block.requires);
        println!("{}", sql_preview);
        println!();
    }

    // // 4) Build dependency graph between blocks and files
    // // We'll map node ids as "file::<filepath>" for file-level, and "file::<filepath>#blockname" for blocks
    // let mut nodes: HashMap<String, (String, Option<String>, String)> = HashMap::new();
    // // key -> (file_path, block_name, sql)
    // let mut deps: HashMap<String, Vec<String>> = HashMap::new();

    // for fb in &all_fileblocks {
    //     let file_node = format!("file:{}", fb.file);
    //     let mut file_requires = vec![];
    //     for r in &fb.requires {
    //         // normalize require to node key; keep as-is for now
    //         file_requires.push(r.clone());
    //     }
    //     nodes.insert(file_node.clone(), (fb.file.clone(), None, String::new()));
    //     deps.insert(file_node.clone(), file_requires);

    //     for block in &fb.blocks {
    //         let key = if let Some(name) = &block.block {
    //             format!("file:{}#{}", fb.file, name)
    //         } else {
    //             format!("file:{}#<file>", fb.file)
    //         };
    //         nodes.insert(key.clone(), (fb.file.clone(), block.block.clone(), block.sql.clone()));
    //         deps.insert(key.clone(), block.requires.clone());
    //     }
    // }

    // // 5) Resolve dependencies naively and execute in order
    // // For simplicity current implementation will execute files/blocks in the order discovered, honoring requires by ensuring dependencies executed first when possible.
    // let pool = db::get_db_pool(config).await?;
    // let mut executed: Vec<String> = vec![];

    // // helper to resolve a requirement token into node keys
    // let resolve_req = |req: &str, fb_file: &str| -> Vec<String> {
    //     let mut out = vec![];
    //     if req.starts_with('/') {
    //         // absolute within version folder: /schema/path
    //         // Resolve to file path under version dir
    //         let path = format!("{}{}", config.sql_base(), req);
    //         out.push(format!("file:{}", path));
    //     } else if req.contains('#') {
    //         // file#block style
    //         // If relative, resolve relative to fb_file
    //         if req.starts_with("file:") {
    //             out.push(req.to_string());
    //         } else {
    //             // relative path
    //             let path = Path::new(&fb_file).parent().unwrap_or(Path::new("")).join(req.split('#').next().unwrap_or(""));
    //             let blk = req.split('#').nth(1).unwrap_or("");
    //             out.push(format!("file:{}#{}", path.to_string_lossy(), blk));
    //         }
    //     } else {
    //         // Could be a filename or block name; try both heuristics
    //         // block name only -> prefer block in same file
    //         out.push(format!("file:{}#{}", fb_file, req));
    //         // Or file relative
    //         let path = Path::new(&fb_file).parent().unwrap_or(Path::new("")).join(req);
    //         out.push(format!("file:{}", path.to_string_lossy()));
    //     }
    //     out
    // };

    // // Execute topologically simple: iterate until all executed or no progress
    // let mut remaining: Vec<String> = nodes.keys().cloned().collect();
    // while !remaining.is_empty() {
    //     let mut progress = false;
    //     let mut still: Vec<String> = vec![];
    //     for node in remaining {
    //         let node_deps = deps.get(&node).cloned().unwrap_or_default();
    //         // translate deps
    //         let mut required_keys: Vec<String> = vec![];
    //         for r in node_deps {
    //             let resolved = resolve_req(&r, &nodes.get(&node).map(|v| v.0.clone()).unwrap_or_default());
    //             for rk in resolved { required_keys.push(rk); }
    //         }

    //         let all_satisfied = required_keys.iter().all(|k| executed.contains(k));
    //         if all_satisfied {
    //             // execute node
    //             if let Some((file, block_name, sql)) = nodes.get(&node) {
    //                 if !sql.trim().is_empty() {
    //                     println!("Executing node {} (file: {}, block: {:?})", node, file, block_name);
    //                     db::execute_sql_string(&pool, sql).await?;
    //                 }
    //             }
    //             executed.push(node.clone());
    //             progress = true;
    //         } else {
    //             still.push(node.clone());
    //         }
    //     }
    //     if !progress {
    //         return Err("Dependency resolution stalled; cyclic or missing dependency".into());
    //     }
    //     remaining = still;
    // }

    println!("Initialization complete");
    Ok(())
}

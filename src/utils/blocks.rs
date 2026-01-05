use serde::{Deserialize, Serialize};
use rand::random;

fn is_valid_relative_block_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let chars: Vec<char> = s.chars().collect();
    if !chars[0].is_alphabetic() && chars[0] != '_' {
        return false;
    }
    for &c in &chars {
        if !c.is_alphanumeric() && c != '_' {
            return false;
        }
    }
    true
}

fn is_valid_absolute_block_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    for part in s.split('.') {
        if !is_valid_relative_block_id(part) {
            return false;
        }
    }
    true
}

fn is_valid_filepath(s: &str) -> bool {
    (s.starts_with("./") || s.starts_with("/")) && !s.contains('\\')
}

fn transform_dep(dep: &str, schema: &str) -> Result<String, String> {
    if is_valid_relative_block_id(dep) {
        Ok(format!("{}.{}", schema, dep))
    } else if is_valid_absolute_block_id(dep) {
        Ok(dep.to_string())
    } else if is_valid_filepath(dep) {
        Ok(dep.to_string())
    } else {
        Err(format!("Invalid dependency: {}", dep))
    }
}

/// Represents a parsed block from SQL content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub schema: String,
    pub file: String,
    pub name: String,
    pub requires: Vec<String>,
    pub sql: String,
}

/// Parse blocks and directives from a SQL file content
pub fn parse_blocks(schema: &str, file_content: &crate::utils::fs::FileContent) -> Result<Vec<Block>, Box<dyn std::error::Error>> {
    let mut file_requires: Vec<String> = vec![];
    let mut blocks: Vec<Block> = vec![];
    let mut block_names: Vec<String> = vec![];

    let mut in_block: Option<String> = None;
    let mut current_sql = String::new();
    let mut current_requires: Vec<String> = vec![];
    let mut file_sql = String::new();

    for line in file_content.content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("-- @requires") {
            // parse requires
            if let Some(req) = trimmed.split_whitespace().nth(2) {
                let dep = req.trim();
                let transformed = transform_dep(dep, schema)?;
                if in_block.is_some() {
                    current_requires.push(transformed);
                } else {
                    file_requires.push(transformed);
                }
            }
            continue;
        }
        if trimmed.starts_with("-- @block") {
            // If there's accumulated file_sql, create a block for it
            if !file_sql.trim().is_empty() {
                let name = format!("block_{:x}", random::<u64>());
                let mut requires = file_requires.clone();
                for bn in &block_names {
                    if is_valid_relative_block_id(bn) && !bn.starts_with("block_") {
                        requires.push(format!("{}.{}", schema, bn));
                    } else {
                        requires.push(bn.clone());
                    }
                }
                let blk = Block {
                    schema: schema.to_string(),
                    file: file_content.path.to_string_lossy().to_string(),
                    name: name.clone(),
                    requires,
                    sql: file_sql.clone(),
                };
                blocks.push(blk);
                block_names.push(name);
                file_sql.clear();
            }
            // start block
            if let Some(name_str) = trimmed.split_whitespace().nth(2) {
                if !is_valid_relative_block_id(name_str) {
                    return Err(format!("Invalid block name: {}", name_str).into());
                }
                in_block = Some(name_str.to_string());
                current_sql.clear();
                current_requires.clear();
            }
            continue;
        }
        if trimmed.starts_with("-- @endblock") {
            // end the block
            let name = if let Some(n) = in_block.clone() {
                n
            } else {
                format!("block_{:x}", random::<u64>())
            };
            let mut requires = file_requires.clone();
            requires.extend(current_requires.clone());
            for bn in &block_names {
                if is_valid_relative_block_id(bn) && !bn.starts_with("block_") {
                    requires.push(format!("{}.{}", schema, bn));
                } else {
                    requires.push(bn.clone());
                }
            }
            let blk = Block {
                schema: schema.to_string(),
                file: file_content.path.to_string_lossy().to_string(),
                name: name.clone(),
                requires,
                sql: current_sql.clone(),
            };
            blocks.push(blk);
            block_names.push(name);
            in_block = None;
            current_sql.clear();
            current_requires.clear();
            continue;
        }

        // Append SQL to current context
        if in_block.is_some() {
            current_sql.push_str(line);
            current_sql.push('\n');
        } else {
            file_sql.push_str(line);
            file_sql.push('\n');
        }
    }

    // If there's remaining file_sql, create a block for it
    if !file_sql.trim().is_empty() {
        let name = format!("block_{:x}", random::<u64>());
        let mut requires = file_requires.clone();
        for bn in &block_names {
            if is_valid_relative_block_id(bn) && !bn.starts_with("block_") {
                requires.push(format!("{}.{}", schema, bn));
            } else {
                requires.push(bn.clone());
            }
        }
        let blk = Block {
            schema: schema.to_string(),
            file: file_content.path.to_string_lossy().to_string(),
            name: name.clone(),
            requires,
            sql: file_sql,
        };
        blocks.push(blk);
        block_names.push(name);
    }

    Ok(blocks)
}

/// Normalize filepaths in block requirements to absolute block identifiers
pub fn normalize(blocks: &mut Vec<Block>, sql_base: &str, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use std::path::Path;

    // Build a map from normalized filepath to list of absolute block IDs
    let mut file_to_blocks: HashMap<String, Vec<String>> = HashMap::new();
    for block in &*blocks {
        let abs_id = format!("{}.{}", block.schema, block.name);
        file_to_blocks.entry(block.file.clone()).or_insert(vec![]).push(abs_id);
    }

    // For each block, transform filepath requires
    for block in blocks {
        let mut new_requires = vec![];
        for req in &block.requires {
            if is_valid_filepath(req) {
                // Resolve the filepath
                let resolved_path = if req.starts_with('/') {
                    // Absolute: already relative to sql_base/version root
                    req.to_string()
                } else if req.starts_with("./") {
                    // Relative: relative to block's file parent
                    let block_path = Path::new(&block.file);
                    let parent = block_path.parent().unwrap_or(Path::new("/"));
                    parent.join(&req[2..]).to_string_lossy().replace('\\', "/")
                } else {
                    continue; // Should not happen due to is_valid_filepath
                };
                // Normalize to POSIX
                let normalized_path = format!("/{}", resolved_path.trim_start_matches('/'));
                // Find matching blocks
                if let Some(block_ids) = file_to_blocks.get(&normalized_path) {
                    new_requires.extend(block_ids.clone());
                } else {
                    return Err(format!("No blocks found for filepath: {}", req).into());
                }
            } else {
                // Keep as is
                new_requires.push(req.clone());
            }
        }
        block.requires = new_requires;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_blocks_example() {
        let content = r#"
-- @requires a
-- @requires b
... SQL OF UNNAMED BLOCK 1 ...

... STILL UNNAMED BLOCK 1 ...
-- @block name
-- @requires c
... SQL OF NAMED BLOCK "name" ...
... This requires c + a,b + unnamed block 1 ...
... there may be empty lines just fine, and many sql statements
-- @endblock

... SQL CODE HERE PERTAINS TO UNNAMED BLOCK 2

-- @block nameT
... this is SQL CODE FOR NAMED BLOCK "nameT" ...
-- @endblock

... SQL CODE HERE PERTAINS TO UNNAMED BLOCK 3
"#;
        let file_content = crate::utils::fs::FileContent {
            path: PathBuf::from("test.sql"),
            content: content.to_string(),
        };
        let blocks = parse_blocks("test_schema", &file_content).unwrap();

        // Expected blocks:
        // 1. Unnamed block 1: requires test_schema.a,test_schema.b
        // 2. Named block "name": requires test_schema.a,test_schema.b,test_schema.c + block_1
        // 3. Unnamed block 2: requires test_schema.a,test_schema.b + block_1,test_schema.name
        // 4. Named block "nameT": requires test_schema.a,test_schema.b + block_1,test_schema.name,block_2
        // 5. Unnamed block 3: requires test_schema.a,test_schema.b + block_1,test_schema.name,block_2,test_schema.nameT

        assert_eq!(blocks.len(), 5);

        // Block 1: unnamed
        assert!(blocks[0].name.starts_with("block_"));
        assert_eq!(blocks[0].requires, vec!["test_schema.a", "test_schema.b"]);

        // Block 2: named "name"
        assert_eq!(blocks[1].name, "name");
        assert_eq!(blocks[1].requires.len(), 4);
        assert!(blocks[1].requires.contains(&"test_schema.a".to_string()));
        assert!(blocks[1].requires.contains(&"test_schema.b".to_string()));
        assert!(blocks[1].requires.contains(&"test_schema.c".to_string()));
        assert!(blocks[1].requires.iter().any(|r| r.starts_with("block_")));

        // Block 3: unnamed
        assert!(blocks[2].name.starts_with("block_"));
        assert_eq!(blocks[2].requires.len(), 4);
        assert!(blocks[2].requires.contains(&"test_schema.a".to_string()));
        assert!(blocks[2].requires.contains(&"test_schema.b".to_string()));
        assert!(blocks[2].requires.contains(&"test_schema.name".to_string()));
        assert!(blocks[2].requires.iter().any(|r| r.starts_with("block_")));

        // Block 4: named "nameT"
        assert_eq!(blocks[3].name, "nameT");
        assert_eq!(blocks[3].requires.len(), 5);
        assert!(blocks[3].requires.contains(&"test_schema.a".to_string()));
        assert!(blocks[3].requires.contains(&"test_schema.b".to_string()));
        assert!(blocks[3].requires.contains(&"test_schema.name".to_string()));
        assert!(blocks[3].requires.iter().filter(|r| r.starts_with("block_")).count() == 2);

        // Block 5: unnamed
        assert!(blocks[4].name.starts_with("block_"));
        assert_eq!(blocks[4].requires.len(), 6);
        assert!(blocks[4].requires.contains(&"test_schema.a".to_string()));
        assert!(blocks[4].requires.contains(&"test_schema.b".to_string()));
        assert!(blocks[4].requires.contains(&"test_schema.name".to_string()));
        assert!(blocks[4].requires.contains(&"test_schema.nameT".to_string()));
        assert!(blocks[4].requires.iter().filter(|r| r.starts_with("block_")).count() == 2);
    }

    #[test]
    fn test_normalize() {
        let mut blocks = vec![
            Block {
                schema: "public".to_string(),
                file: "/public/schema.sql".to_string(),
                name: "block1".to_string(),
                requires: vec!["./other.sql".to_string()],
                sql: "SELECT 1;".to_string(),
            },
            Block {
                schema: "public".to_string(),
                file: "/public/other.sql".to_string(),
                name: "block2".to_string(),
                requires: vec![],
                sql: "SELECT 2;".to_string(),
            },
            Block {
                schema: "public".to_string(),
                file: "/public/third.sql".to_string(),
                name: "block3".to_string(),
                requires: vec!["/public/other.sql".to_string()],
                sql: "SELECT 3;".to_string(),
            },
        ];

        normalize(&mut blocks, "sql", "next").unwrap();

        assert_eq!(blocks[0].requires, vec!["public.block2".to_string()]);
        assert_eq!(blocks[1].requires, Vec::<String>::new());
        assert_eq!(blocks[2].requires, vec!["public.block2".to_string()]);
    }
}

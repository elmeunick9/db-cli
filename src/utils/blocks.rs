use serde::{Deserialize, Serialize};
use rand::random;

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
                let dep = req.trim().to_string();
                if in_block.is_some() {
                    current_requires.push(dep);
                } else {
                    file_requires.push(dep);
                }
            }
            continue;
        }
        if trimmed.starts_with("-- @block") {
            // If there's accumulated file_sql, create a block for it
            if !file_sql.trim().is_empty() {
                let name = format!("block_{:x}", random::<u64>());
                let mut requires = file_requires.clone();
                requires.extend(block_names.clone());
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
            if let Some(name) = trimmed.split_whitespace().nth(2) {
                in_block = Some(name.to_string());
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
            requires.extend(block_names.clone());
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
        requires.extend(block_names.clone());
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
        // 1. Unnamed block 1: requires a,b
        // 2. Named block "name": requires a,b,c + block_1
        // 3. Unnamed block 2: requires a,b + block_1,block_name
        // 4. Named block "nameT": requires a,b + block_1,block_name,block_2
        // 5. Unnamed block 3: requires a,b + block_1,block_name,block_2,nameT

        assert_eq!(blocks.len(), 5);

        // Block 1: unnamed
        assert!(blocks[0].name.starts_with("block_"));
        assert_eq!(blocks[0].requires, vec!["a", "b"]);

        // Block 2: named "name"
        assert_eq!(blocks[1].name, "name");
        assert_eq!(blocks[1].requires.len(), 4);
        assert!(blocks[1].requires.contains(&"a".to_string()));
        assert!(blocks[1].requires.contains(&"b".to_string()));
        assert!(blocks[1].requires.contains(&"c".to_string()));
        assert!(blocks[1].requires.iter().any(|r| r.starts_with("block_")));

        // Block 3: unnamed
        assert!(blocks[2].name.starts_with("block_"));
        assert_eq!(blocks[2].requires.len(), 4);
        assert!(blocks[2].requires.contains(&"a".to_string()));
        assert!(blocks[2].requires.contains(&"b".to_string()));
        assert!(blocks[2].requires.contains(&"name".to_string()));
        assert!(blocks[2].requires.iter().any(|r| r.starts_with("block_")));

        // Block 4: named "nameT"
        assert_eq!(blocks[3].name, "nameT");
        assert_eq!(blocks[3].requires.len(), 5);
        assert!(blocks[3].requires.contains(&"a".to_string()));
        assert!(blocks[3].requires.contains(&"b".to_string()));
        assert!(blocks[3].requires.contains(&"name".to_string()));
        assert!(blocks[3].requires.iter().filter(|r| r.starts_with("block_")).count() == 2);

        // Block 5: unnamed
        assert!(blocks[4].name.starts_with("block_"));
        assert_eq!(blocks[4].requires.len(), 6);
        assert!(blocks[4].requires.contains(&"a".to_string()));
        assert!(blocks[4].requires.contains(&"b".to_string()));
        assert!(blocks[4].requires.contains(&"name".to_string()));
        assert!(blocks[4].requires.contains(&"nameT".to_string()));
        assert!(blocks[4].requires.iter().filter(|r| r.starts_with("block_")).count() == 2);
    }
}

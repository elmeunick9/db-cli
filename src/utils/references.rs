use crate::config::{Config, ReferenceValue};

/// Get a reference by key and format it according to the SQL dialect
/// Reference format: ["schema", "object_name", ["column1", "column2", ...]]
/// Where object_name can be a table, view, function, enum, index, trigger, etc.
/// Schema is always required; columns are optional.
/// 
/// PostgreSQL output: "schema"."object" (column1, column2, ...)
/// MySQL output: `schema`.`object` (column1, column2, ...)
/// SQLite output: schema.object (column1, column2, ...)
pub fn get_reference(config: &Config, key: &str) -> Result<String, String> {
    let ref_value = config.references.get(key)
        .ok_or_else(|| format!("Reference '{}' not found", key))?;

    // Parse the reference structure: ["schema", "object", [...columns]]
    let (schema, object, columns) = parse_reference_value(ref_value)?;

    match config.sql_dialect.as_str() {
        "postgres" | "postgresql" => {
            if let Some(cols) = columns {
                let columns_str = cols.join(", ");
                Ok(format!("\"{}\".\"{}\" ({})", schema, object, columns_str))
            } else {
                Ok(format!("\"{}\".\"{}\"", schema, object))
            }
        },
        "mysql" => {
            if let Some(cols) = columns {
                let columns_str = cols.join(", ");
                Ok(format!("`{}`.`{}` ({})", schema, object, columns_str))
            } else {
                Ok(format!("`{}`.`{}`", schema, object))
            }
        },
        "sqlite" => {
            let qualified_name = format!("{}.{}", schema, object);
            if let Some(cols) = columns {
                let columns_str = cols.join(", ");
                Ok(format!("{} ({})", qualified_name, columns_str))
            } else {
                Ok(qualified_name)
            }
        },
        _ => {
            Err(format!("Unknown SQL dialect: {}", config.sql_dialect))
        }
    }
}

/// Parse a ReferenceValue into (schema, object, optional_columns)
/// Expects format: ["schema", "object"] or ["schema", "object", ["col1", "col2", ...]]
/// Schema and object are required.
pub fn parse_reference_value(ref_value: &ReferenceValue) -> Result<(String, String, Option<Vec<String>>), String> {
    match ref_value {
        ReferenceValue::StringArray(arr) => {
            if arr.len() < 2 {
                return Err("Reference must contain at least schema and object name".to_string());
            }
            
            let schema = arr[0].clone();
            let object = arr[1].clone();
            let columns = if arr.len() > 2 {
                Some(arr[2..].to_vec())
            } else {
                None
            };
            
            Ok((schema, object, columns))
        },
        ReferenceValue::NestedArray(arr) => {
            if arr.len() < 2 {
                return Err("Reference must contain at least schema and object name".to_string());
            }
            
            let schema = match &arr[0] {
                ReferenceValue::String(s) => s.clone(),
                _ => return Err("First element (schema) must be a string".to_string()),
            };
            
            let object = match &arr[1] {
                ReferenceValue::String(s) => s.clone(),
                _ => return Err("Second element (object name) must be a string".to_string()),
            };
            
            let columns = if arr.len() > 2 {
                match &arr[2] {
                    ReferenceValue::StringArray(cols) => Some(cols.clone()),
                    _ => return Err("Third element (columns) must be an array of strings".to_string()),
                }
            } else {
                None
            };
            
            Ok((schema, object, columns))
        },
        ReferenceValue::String(_) => {
            Err("Reference must be an array, not a single string".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_reference_postgres_with_columns() {
        let mut config = Config::default();
        config.sql_dialect = "postgres".to_string();
        config.references.insert("meta_key".to_string(), ReferenceValue::NestedArray(vec![
            ReferenceValue::String("public".to_string()),
            ReferenceValue::String("meta".to_string()),
            ReferenceValue::StringArray(vec!["key".to_string()]),
        ]));

        let result = get_reference(&config, "meta_key").unwrap();
        assert_eq!(result, "\"public\".\"meta\" (key)");
    }

    #[test]
    fn test_get_reference_postgres_no_columns() {
        let mut config = Config::default();
        config.sql_dialect = "postgres".to_string();
        config.references.insert("table_ref".to_string(), ReferenceValue::StringArray(vec![
            "public".to_string(),
            "users".to_string(),
        ]));

        let result = get_reference(&config, "table_ref").unwrap();
        assert_eq!(result, "\"public\".\"users\"");
    }

    #[test]
    fn test_get_reference_mysql() {
        let mut config = Config::default();
        config.sql_dialect = "mysql".to_string();
        config.references.insert("meta_key".to_string(), ReferenceValue::NestedArray(vec![
            ReferenceValue::String("public".to_string()),
            ReferenceValue::String("meta".to_string()),
            ReferenceValue::StringArray(vec!["key".to_string()]),
        ]));

        let result = get_reference(&config, "meta_key").unwrap();
        assert_eq!(result, "`public`.`meta` (key)");
    }

    #[test]
    fn test_get_reference_sqlite() {
        let mut config = Config::default();
        config.sql_dialect = "sqlite".to_string();
        config.references.insert("table_ref".to_string(), ReferenceValue::StringArray(vec![
            "main".to_string(),
            "users".to_string(),
        ]));

        let result = get_reference(&config, "table_ref").unwrap();
        assert_eq!(result, "main.users");
    }

    #[test]
    fn test_get_reference_not_found() {
        let config = Config::default();
        let result = get_reference(&config, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_parse_reference_missing_object() {
        let ref_value = ReferenceValue::StringArray(vec!["public".to_string()]);
        let result = parse_reference_value(&ref_value);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least schema and object name"));
    }
}

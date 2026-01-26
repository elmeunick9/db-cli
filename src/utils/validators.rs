/// Validate that a key is in proper snake_case format (lowercase, alphanumeric, underscores only)
pub fn validate_key_format(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("Key cannot be empty".to_string());
    }

    // Check if it matches snake_case pattern: lowercase, digits, underscores
    // Must start with lowercase letter, can contain underscores and digits
    if !key.chars().next().unwrap().is_lowercase() {
        return Err(format!("Key '{}' must start with a lowercase letter", key));
    }

    for c in key.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
            return Err(format!(
                "Key '{}' contains invalid character '{}'. Keys must be in snake_case: lowercase letters, digits, and underscores only",
                key, c
            ));
        }
    }

    if key.ends_with('_') || key.starts_with('_') {
        return Err(format!("Key '{}' cannot start or end with underscore", key));
    }

    if key.contains("__") {
        return Err(format!("Key '{}' cannot contain consecutive underscores", key));
    }

    Ok(())
}

/// Validate that a SQL dialect is supported
pub fn validate_dialect(dialect: &str) -> Result<(), String> {
    match dialect {
        "postgres" | "postgresql" | "mysql" | "mssql" | "sqlite" => Ok(()),
        _ => Err(format!(
            "Unsupported SQL dialect '{}'. Supported dialects: postgres, postgresql, mysql, mssql, sqlite",
            dialect
        )),
    }
}